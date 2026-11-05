/// A plain `X.Y.Z` release version; pre-release and build suffixes do not parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AppVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

const MAX_DIGITS: usize = 9;

impl AppVersion {
    pub fn parse(text: &str) -> Option<AppVersion> {
        let mut parts = text.split('.');
        let major = parse_part(parts.next()?)?;
        let minor = parse_part(parts.next()?)?;
        let patch = parse_part(parts.next()?)?;
        if parts.next().is_some() {
            return None;
        }
        Some(AppVersion {
            major,
            minor,
            patch,
        })
    }
}

fn parse_part(part: &str) -> Option<u32> {
    let digits_only = !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    let leading_zero = part.len() > 1 && part.starts_with('0');
    if !digits_only || leading_zero || part.len() > MAX_DIGITS {
        return None;
    }
    part.parse().ok()
}

/// Only a parsable remote version above a parsable current one is offered: pre-releases never
/// are, and a current version that does not parse is never updated.
pub fn is_offered(current: &str, remote: &str) -> bool {
    match (AppVersion::parse(current), AppVersion::parse(remote)) {
        (Some(current), Some(remote)) => remote > current,
        _ => false,
    }
}
