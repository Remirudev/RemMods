//! Reads a procfs-shaped tree. No cfg: the real `/proc` on Linux, a fixture tree in tests on
//! every OS.

use std::fs;
use std::io;
use std::path::Path;

/// One process as the OS reports it. Text is lossy UTF-8; only ASCII names are ever matched.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProcessRecord {
    pub pid: u32,
    /// Linux: `/proc/<pid>/comm` without its trailing '\n' (lossy UTF-8).
    /// Windows: `PROCESSENTRY32W.szExeFile` up to the first NUL (lossy).
    pub name: String,
    /// Linux: `/proc/<pid>/cmdline`, trailing NUL padding dropped, split on NUL, lossy UTF-8;
    /// empty for kernel threads and zombies. Windows: always empty.
    pub argv: Vec<String>,
}

/// Every numeric entry of `root` that is still a process, sorted by pid. Entries that vanish
/// while they are read are skipped: processes come and go during a scan.
pub fn read_proc_tree(root: &Path) -> io::Result<Vec<ProcessRecord>> {
    let mut records = Vec::new();
    for entry in fs::read_dir(root)? {
        let Ok(entry) = entry else { continue };
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if let Some(record) = read_process(&entry.path(), pid) {
            records.push(record);
        }
    }
    records.sort_unstable_by_key(|record| record.pid);
    Ok(records)
}

/// `None` when the process is gone (no `comm`, or `cmdline` vanished after `comm` was read).
fn read_process(dir: &Path, pid: u32) -> Option<ProcessRecord> {
    let name = parse_comm(&fs::read(dir.join("comm")).ok()?);
    let argv = match fs::read(dir.join("cmdline")) {
        Ok(raw) => parse_cmdline(&raw),
        Err(error) if is_gone(&error) => return None,
        Err(error) => {
            log::debug!("cmdline of pid {pid} unreadable: {error}");
            Vec::new()
        }
    };
    Some(ProcessRecord { pid, name, argv })
}

/// ENOENT (gone before the open) or ESRCH (gone between open and read).
fn is_gone(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::NotFound || super::imp::is_no_such_process(error)
}

/// Wine shifts argv in place and zero-fills the tail, so trailing NULs are padding, not
/// arguments. Interior empty arguments are kept.
pub fn parse_cmdline(raw: &[u8]) -> Vec<String> {
    let Some(last) = raw.iter().rposition(|byte| *byte != 0) else {
        return Vec::new();
    };
    raw[..=last]
        .split(|byte| *byte == 0)
        .map(|arg| String::from_utf8_lossy(arg).into_owned())
        .collect()
}

/// The kernel ends `comm` with one '\n'; only that one is dropped.
pub fn parse_comm(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw.strip_suffix(b"\n").unwrap_or(raw)).into_owned()
}
