//! `utoc-list <dir>`: read-only listing of every `.utoc`/`.utoc-x` below `<dir>` (SPEC-P2B §4.7),
//! cross-checked against `scripts/utoc-list.py`.

use std::io::Write;
use std::path::{Path, PathBuf};

use remmods_pak::{PakError, read_toc};

fn is_container(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".utoc") || lower.ends_with(".utoc-x")
}

fn status(error: &PakError) -> &'static str {
    match error {
        PakError::Io(_) => "unreadable",
        PakError::NotIoStore => "notIoStore",
        PakError::UnsupportedVersion(_) => "unsupportedVersion",
        PakError::Encrypted => "encrypted",
        PakError::NoDirectoryIndex => "noIndex",
        PakError::Malformed(_) => "malformed",
        PakError::TooLarge => "tooLarge",
    }
}

/// Symlinked folders are not followed; a link to a regular file counts as a file.
fn collect(dir: &Path, rel: &str, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            collect(&path, &child_rel, out);
        } else if is_container(&name) && std::fs::metadata(&path).is_ok_and(|m| m.is_file()) {
            out.push((child_rel, path));
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [dir] = args.as_slice() else {
        eprintln!("usage: utoc-list <dir>");
        std::process::exit(2);
    };
    let root = Path::new(dir);
    if !root.is_dir() {
        eprintln!("utoc-list: not a directory: {dir}");
        std::process::exit(2);
    }
    let mut containers = Vec::new();
    collect(root, "", &mut containers);
    let mut lines = Vec::new();
    for (rel, path) in containers {
        match read_toc(&path) {
            Ok(listing) => lines.extend(
                listing
                    .files
                    .into_iter()
                    .map(|file| format!("{rel}\t{}", file.path)),
            ),
            Err(error) => lines.push(format!("{rel}\t!{}", status(&error))),
        }
    }
    lines.sort();
    let mut stdout = std::io::stdout().lock();
    for line in lines {
        if writeln!(stdout, "{line}").is_err() {
            break;
        }
    }
}
