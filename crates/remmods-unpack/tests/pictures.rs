//! `render_picture`: which mip is used and what comes out.
#![allow(clippy::expect_used)]

use remmods_unpack::fixture::{MipSpec, bgra_package, texture_package};
use remmods_unpack::{UnpackError, render_picture};

const OPTIONAL_PAYLOAD: u32 = 0x800;

struct Decoded {
    width: u32,
    height: u32,
    color: png::ColorType,
    pixels: Vec<u8>,
}

fn decode(png_bytes: &[u8]) -> Decoded {
    let mut reader = png::Decoder::new(std::io::Cursor::new(png_bytes))
        .read_info()
        .expect("png header");
    let mut pixels = vec![0; reader.output_buffer_size().expect("size")];
    let info = reader.next_frame(&mut pixels).expect("png data");
    pixels.truncate(info.buffer_size());
    Decoded {
        width: info.width,
        height: info.height,
        color: info.color_type,
        pixels,
    }
}

fn inline(width: i32, height: i32, payload: &[u8]) -> MipSpec<'_> {
    MipSpec {
        width,
        height,
        payload,
        bulk_at: None,
        extra_flags: 0,
    }
}

fn bgra(width: usize, height: usize, color: [u8; 4]) -> Vec<u8> {
    color.repeat(width * height)
}

#[test]
fn bgra_pixels_come_out_as_rgb() {
    let picture = render_picture(&bgra_package([10, 20, 30, 255]), None, 64).expect("render");
    let decoded = decode(&picture.png);
    assert_eq!((picture.width, picture.height), (2, 2));
    assert_eq!((decoded.width, decoded.height), (2, 2));
    assert_eq!(decoded.color, png::ColorType::Rgb);
    assert_eq!(decoded.pixels, [30, 20, 10].repeat(4));
}

#[test]
fn transparency_is_kept() {
    let picture = render_picture(&bgra_package([10, 20, 30, 128]), None, 64).expect("render");
    let decoded = decode(&picture.png);
    assert_eq!(decoded.color, png::ColorType::Rgba);
    assert_eq!(decoded.pixels, [30, 20, 10, 128].repeat(4));
}

#[test]
fn block_formats_decode() {
    // One DXT1 block: both end colours red in RGB565, all pixels use the first.
    let dxt1 = [0x00, 0xF8, 0x00, 0xF8, 0, 0, 0, 0];
    let package = texture_package("PF_DXT1", &[inline(4, 4, &dxt1)]);
    let decoded = decode(&render_picture(&package, None, 64).expect("dxt1").png);
    assert_eq!((decoded.width, decoded.height), (4, 4));
    assert_eq!(&decoded.pixels[..3], [255, 0, 0]);

    let bc7 = [0u8; 16];
    let package = texture_package("PF_BC7", &[inline(4, 4, &bc7)]);
    assert!(render_picture(&package, None, 64).is_ok());

    let dxt5 = [
        255, 255, 0, 0, 0, 0, 0, 0, 0x00, 0xF8, 0x00, 0xF8, 0, 0, 0, 0,
    ];
    let package = texture_package("PF_DXT5", &[inline(4, 4, &dxt5)]);
    let decoded = decode(&render_picture(&package, None, 64).expect("dxt5").png);
    assert_eq!(&decoded.pixels[..3], [255, 0, 0]);
}

#[test]
fn the_smallest_mip_that_covers_the_edge_is_used() {
    let large = bgra(8, 8, [0, 0, 255, 255]);
    let middle = bgra(4, 4, [0, 255, 0, 255]);
    let small = bgra(2, 2, [255, 0, 0, 255]);
    let package = texture_package(
        "PF_B8G8R8A8",
        &[
            inline(8, 8, &large),
            inline(4, 4, &middle),
            inline(2, 2, &small),
        ],
    );
    let decoded = decode(&render_picture(&package, None, 4).expect("render").png);
    assert_eq!((decoded.width, decoded.height), (4, 4));
    assert_eq!(&decoded.pixels[..3], [0, 255, 0]);
}

#[test]
fn a_larger_mip_is_shrunk_to_the_edge() {
    let wide = bgra(8, 4, [0, 255, 0, 255]);
    let package = texture_package("PF_B8G8R8A8", &[inline(8, 4, &wide)]);
    let decoded = decode(&render_picture(&package, None, 4).expect("render").png);
    assert_eq!((decoded.width, decoded.height), (4, 2));
    assert_eq!(decoded.pixels, [0, 255, 0].repeat(8));
}

#[test]
fn shrinking_leaves_no_dark_fringe_around_transparent_pixels() {
    let mut pixels = bgra(2, 1, [0, 0, 255, 255]);
    pixels[4..].fill(0);
    let package = texture_package("PF_B8G8R8A8", &[inline(2, 1, &pixels)]);
    let decoded = decode(&render_picture(&package, None, 1).expect("render").png);
    assert_eq!(decoded.pixels, [255, 0, 0, 127]);
}

#[test]
fn mips_in_the_bulk_file_are_read_from_it() {
    let pixels = bgra(4, 4, [1, 2, 3, 255]);
    let mut bulk = vec![0xEE; 16];
    bulk.extend_from_slice(&pixels);
    let package = texture_package(
        "PF_B8G8R8A8",
        &[MipSpec {
            width: 4,
            height: 4,
            payload: &pixels,
            bulk_at: Some(16),
            extra_flags: 0,
        }],
    );
    let decoded = decode(
        &render_picture(&package, Some(&bulk), 4)
            .expect("render")
            .png,
    );
    assert_eq!(&decoded.pixels[..3], [3, 2, 1]);
}

#[test]
fn a_mip_that_cannot_be_read_is_passed_over() {
    let large = bgra(8, 8, [0, 0, 255, 255]);
    let small = bgra(2, 2, [255, 0, 0, 255]);
    let unreadable = [
        MipSpec {
            width: 8,
            height: 8,
            payload: &large,
            bulk_at: Some(0),
            extra_flags: 0,
        },
        inline(2, 2, &small),
    ];
    let package = texture_package("PF_B8G8R8A8", &unreadable);
    let decoded = decode(&render_picture(&package, None, 8).expect("no bulk").png);
    assert_eq!(decoded.width, 2);

    let optional = [
        MipSpec {
            extra_flags: OPTIONAL_PAYLOAD,
            ..unreadable[0].clone()
        },
        unreadable[1].clone(),
    ];
    let package = texture_package("PF_B8G8R8A8", &optional);
    let decoded = decode(
        &render_picture(&package, Some(&large), 8)
            .expect("optional")
            .png,
    );
    assert_eq!(decoded.width, 2);
}

#[test]
fn other_assets_are_no_textures() {
    assert!(matches!(
        render_picture(b"\0\0\0\0short", None, 64),
        Err(UnpackError::Malformed(_))
    ));
    assert!(matches!(
        render_picture(&[1; 64], None, 64),
        Err(UnpackError::NotATexture)
    ));
}

#[test]
fn an_unknown_pixel_format_is_named() {
    let package = texture_package("PF_ASTC_4x4", &[inline(4, 4, &[0; 16])]);
    assert!(matches!(
        render_picture(&package, None, 64),
        Err(UnpackError::UnsupportedFormat(format)) if format == "PF_ASTC_4x4"
    ));
}

#[test]
fn a_mip_shorter_than_its_size_is_refused() {
    let package = texture_package("PF_B8G8R8A8", &[inline(4, 4, &[0; 16])]);
    assert!(matches!(
        render_picture(&package, None, 64),
        Err(UnpackError::NotATexture)
    ));
}

#[test]
fn garbled_packages_never_panic() {
    let pixels = bgra(4, 4, [1, 2, 3, 255]);
    let dxt = [0x55u8; 32];
    let packages = [
        texture_package(
            "PF_B8G8R8A8",
            &[inline(4, 4, &pixels), inline(2, 2, &pixels[..16])],
        ),
        texture_package("PF_BC7", &[inline(4, 4, &dxt[..16])]),
        texture_package("PF_DXT1", &[inline(8, 4, &dxt[..16])]),
    ];
    let mut seed = 0x9E37_79B9_u32;
    for package in &packages {
        for _ in 0..3000 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let mut bytes = package.clone();
            let at = usize::try_from(seed).unwrap_or(0) % bytes.len();
            bytes[at] = u8::try_from(seed >> 24).unwrap_or(0);
            let _ = render_picture(&bytes, Some(&dxt), 64);
        }
    }
}
