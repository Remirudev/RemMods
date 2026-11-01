/// Raw index path → display path: leading `../` and then leading `/` removed (§4.4).
pub fn display_path(raw: &str) -> String {
    let mut rest = raw;
    while let Some(stripped) = rest.strip_prefix("../") {
        rest = stripped;
    }
    rest.trim_start_matches('/').to_owned()
}

/// Display path with ASCII letters lowercased; engine package names are case-insensitive.
pub fn file_key(display: &str) -> String {
    display.to_ascii_lowercase()
}

/// File key without the last extension of the final segment, so `.uasset` and `.ubulk` of one
/// package share a key.
pub fn asset_key(display: &str) -> String {
    let key = file_key(display);
    let segment_start = key.rfind('/').map_or(0, |slash| slash + 1);
    match key[segment_start..].rfind('.') {
        Some(dot) if dot > 0 => key[..segment_start + dot].to_owned(),
        _ => key,
    }
}
