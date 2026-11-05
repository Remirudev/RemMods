//! Name fixes for names RemMods would create (spec §5.3 step 9).

use std::ffi::OsStr;

use super::detect_root::is_live_payload_name;
use crate::safe_fs::names::{
    NAME_MAX_BYTES, NameIssue, PAYLOAD_NAME_MAX_BYTES, has_disabled_prefix_folded, is_bidi_control,
    is_install_temp_name, is_reserved_device_name, validate_name,
};

const FORBIDDEN: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// The issue of a file component RemMods would create: `validate_name`, plus payload names longer
/// than 253 bytes (their staged `-x` form must fit 255).
pub(crate) fn file_name_issue(name: &str) -> Option<NameIssue> {
    if let Err(issue) = validate_name(name) {
        return Some(issue);
    }
    (is_live_payload_name(name) && name.len() > PAYLOAD_NAME_MAX_BYTES)
        .then_some(NameIssue::TooLong)
}

/// The issue of a candidate (mod folder) name; `.disabled_` and install-temp names included.
pub(crate) fn folder_name_issue(name: &str) -> Option<NameIssue> {
    validate_name(name).err()
}

/// Forbidden, control and bidi characters → `_`, trailing dots and spaces stripped, `_` after a
/// reserved device stem, cut at a char boundary (extension kept) to 255 bytes, 253 for live
/// payload names. Never empty.
pub fn fix_name(name: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|c| {
            if FORBIDDEN.contains(&c) || c.is_control() || is_bidi_control(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = replaced.trim_end_matches(['.', ' ']);
    let mut fixed = if trimmed.is_empty() {
        String::from("_")
    } else {
        trimmed.to_owned()
    };
    if is_reserved_device_name(&fixed) {
        fixed = match fixed.find('.') {
            Some(dot) => format!("{}_{}", &fixed[..dot], &fixed[dot..]),
            None => format!("{fixed}_"),
        };
    }
    let budget = if is_live_payload_name(&fixed) {
        PAYLOAD_NAME_MAX_BYTES
    } else {
        NAME_MAX_BYTES
    };
    cut_keeping_extension(&fixed, budget)
}

/// `fix_name` plus the folder rules: a leading `.disabled_` loses its dot, an install-temp name
/// loses its leading `_`.
pub(crate) fn fix_folder_name(name: &str) -> String {
    let mut fixed = fix_name(name);
    if has_disabled_prefix_folded(&fixed)
        && let Some(rest) = fixed.strip_prefix('.')
    {
        fixed = rest.to_owned();
    }
    if is_install_temp_name(OsStr::new(&fixed))
        && let Some(rest) = fixed.strip_prefix('_')
    {
        fixed = rest.to_owned();
    }
    fixed
}

/// Cuts the stem so the whole name fits `budget` bytes; an extension longer than half the budget
/// is cut with the rest.
fn cut_keeping_extension(name: &str, budget: usize) -> String {
    if name.len() <= budget {
        return name.to_owned();
    }
    let (stem, extension) = match name.rfind('.') {
        Some(dot) if dot > 0 && name.len() - dot <= budget / 2 => name.split_at(dot),
        _ => (name, ""),
    };
    let keep = budget - extension.len();
    format!("{}{extension}", cut_at_char(stem, keep))
}

fn cut_at_char(text: &str, max_bytes: usize) -> &str {
    let mut end = text.len().min(max_bytes);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.get(..end).unwrap_or_default()
}

/// The stem before the last `.` (the whole name without one).
pub(crate) fn stem_of(name: &str) -> &str {
    match name.rfind('.') {
        Some(dot) if dot > 0 => &name[..dot],
        _ => name,
    }
}

/// The extension from the last `.` on ("" without one).
pub(crate) fn extension_of(name: &str) -> &str {
    match name.rfind('.') {
        Some(dot) if dot > 0 => &name[dot..],
        _ => "",
    }
}
