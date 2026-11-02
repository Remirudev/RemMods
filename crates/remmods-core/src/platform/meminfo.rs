//! `/proc/meminfo` parsing, cfg-free so it is unit-tested on every OS.

/// `MemTotal` in bytes (the file states kB, meaning KiB); None when absent or malformed.
pub fn parse_meminfo_total(text: &str) -> Option<u64> {
    let line = text
        .lines()
        .find_map(|line| line.strip_prefix("MemTotal:"))?;
    let mut fields = line.split_whitespace();
    let value: u64 = fields.next()?.parse().ok()?;
    match fields.next() {
        Some("kB") => value.checked_mul(1024),
        _ => None,
    }
}
