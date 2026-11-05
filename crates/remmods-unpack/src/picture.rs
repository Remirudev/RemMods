//! From a cooked texture to a PNG: block decoding, shrinking and encoding.

use png::{BitDepth, ColorType, Compression, Encoder};
use texture2ddecoder::{decode_bc1, decode_bc3, decode_bc7};

use crate::UnpackError;
use crate::texture::{self, PixelFormat};

const CHANNELS: usize = 4;
const BC1_BLOCK: usize = 8;
const BC_BLOCK: usize = 16;

/// A picture ready to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// RGBA8, row by row.
struct Pixels {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

fn blocks(edge: usize) -> usize {
    edge.div_ceil(4)
}

fn block_bytes(format: PixelFormat, width: usize, height: usize) -> Option<usize> {
    let area = blocks(width).checked_mul(blocks(height))?;
    match format {
        PixelFormat::Dxt1 => area.checked_mul(BC1_BLOCK),
        PixelFormat::Dxt5 | PixelFormat::Bc7 => area.checked_mul(BC_BLOCK),
        PixelFormat::Bgra8 => width.checked_mul(height)?.checked_mul(CHANNELS),
    }
}

/// The decoders write BGRA words; either way the bytes come out as RGBA.
fn bgra_to_rgba(words: &[u32]) -> Vec<u8> {
    words
        .iter()
        .flat_map(|word| {
            let [blue, green, red, alpha] = word.to_le_bytes();
            [red, green, blue, alpha]
        })
        .collect()
}

fn decode(format: PixelFormat, width: usize, height: usize, payload: &[u8]) -> Option<Vec<u8>> {
    let needed = block_bytes(format, width, height)?;
    let payload = payload.get(..needed)?;
    if format == PixelFormat::Bgra8 {
        let swapped = payload
            .as_chunks::<CHANNELS>()
            .0
            .iter()
            .flat_map(|&[blue, green, red, alpha]| [red, green, blue, alpha]);
        return Some(swapped.collect());
    }
    let mut words = vec![0u32; width.checked_mul(height)?];
    let decoded = match format {
        PixelFormat::Dxt1 => decode_bc1(payload, width, height, &mut words),
        PixelFormat::Dxt5 => decode_bc3(payload, width, height, &mut words),
        _ => decode_bc7(payload, width, height, &mut words),
    };
    decoded.ok()?;
    Some(bgra_to_rgba(&words))
}

/// Area average weighted by alpha, so transparent pixels leave no dark fringe.
fn shrink(source: &Pixels, width: usize, height: usize) -> Pixels {
    let mut data = Vec::with_capacity(width * height * CHANNELS);
    for y in 0..height {
        let rows = (y * source.height / height)..((y + 1) * source.height / height).max(1);
        for x in 0..width {
            let columns = (x * source.width / width)..((x + 1) * source.width / width).max(1);
            data.extend_from_slice(&average(source, rows.clone(), columns));
        }
    }
    Pixels {
        width,
        height,
        data,
    }
}

fn average(
    source: &Pixels,
    rows: std::ops::Range<usize>,
    columns: std::ops::Range<usize>,
) -> [u8; CHANNELS] {
    let (mut red, mut green, mut blue, mut alpha, mut count) = (0u64, 0u64, 0u64, 0u64, 0u64);
    for row in rows {
        for column in columns.clone() {
            let at = (row * source.width + column) * CHANNELS;
            let Some(&[r, g, b, a]) = source
                .data
                .get(at..at + CHANNELS)
                .and_then(|p| p.first_chunk())
            else {
                continue;
            };
            let weight = u64::from(a);
            red += u64::from(r) * weight;
            green += u64::from(g) * weight;
            blue += u64::from(b) * weight;
            alpha += weight;
            count += 1;
        }
    }
    if alpha == 0 || count == 0 {
        return [0; CHANNELS];
    }
    let channel = |sum: u64| u8::try_from(sum / alpha).unwrap_or(u8::MAX);
    [
        channel(red),
        channel(green),
        channel(blue),
        u8::try_from(alpha / count).unwrap_or(u8::MAX),
    ]
}

fn fitted(pixels: Pixels, max_edge: usize) -> Pixels {
    let longest = pixels.width.max(pixels.height);
    if longest <= max_edge {
        return pixels;
    }
    let scale = |edge: usize| (edge * max_edge / longest).max(1);
    let (width, height) = (scale(pixels.width), scale(pixels.height));
    shrink(&pixels, width, height)
}

fn encode(pixels: &Pixels) -> Result<Vec<u8>, UnpackError> {
    let edge = |value: usize| u32::try_from(value).map_err(|_| UnpackError::Encode);
    let opaque = pixels
        .data
        .as_chunks::<CHANNELS>()
        .0
        .iter()
        .all(|p| p[3] == u8::MAX);
    let mut out = Vec::new();
    let mut encoder = Encoder::new(&mut out, edge(pixels.width)?, edge(pixels.height)?);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_compression(Compression::Fast);
    encoder.set_color(if opaque {
        ColorType::Rgb
    } else {
        ColorType::Rgba
    });
    let mut writer = encoder.write_header().map_err(|_| UnpackError::Encode)?;
    let written = if opaque {
        let rgb: Vec<u8> = pixels
            .data
            .as_chunks::<CHANNELS>()
            .0
            .iter()
            .flat_map(|&[r, g, b, _]| [r, g, b])
            .collect();
        writer.write_image_data(&rgb)
    } else {
        writer.write_image_data(&pixels.data)
    };
    written.map_err(|_| UnpackError::Encode)?;
    writer.finish().map_err(|_| UnpackError::Encode)?;
    Ok(out)
}

/// A PNG of the texture in `package`, its longest edge at most `max_edge`. `bulk` is the asset's
/// `.ubulk` chunk, for mips stored there.
pub fn render_picture(
    package: &[u8],
    bulk: Option<&[u8]>,
    max_edge: u32,
) -> Result<Picture, UnpackError> {
    let texture = texture::parse(package, bulk)?;
    let (mip, payload) = texture.best_mip(max_edge).ok_or(UnpackError::NotATexture)?;
    let size = |edge: u32| usize::try_from(edge).map_err(|_| UnpackError::NotATexture);
    let (width, height) = (size(mip.width)?, size(mip.height)?);
    if width == 0 || height == 0 {
        return Err(UnpackError::NotATexture);
    }
    let data = decode(texture.format, width, height, payload).ok_or(UnpackError::NotATexture)?;
    let pixels = fitted(
        Pixels {
            width,
            height,
            data,
        },
        size(max_edge.max(1))?,
    );
    Ok(Picture {
        png: encode(&pixels)?,
        width: u32::try_from(pixels.width).map_err(|_| UnpackError::Encode)?,
        height: u32::try_from(pixels.height).map_err(|_| UnpackError::Encode)?,
    })
}
