#![allow(clippy::expect_used, clippy::disallowed_methods, dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use remmods_core::AppPaths;
use remmods_core::platform::{self, TopdirPolicy, TrashEnv};
use remmods_core::safe_fs::{BatchOptions, FaultPlan, OpGate, RenameOp, RetryMode, Scope};
use tempfile::TempDir;

/// A temp dir and its canonical path. Windows runners hand out 8.3 aliases (`RUNNER~1`), so tests
/// compare against the canonical form only.
pub fn temp_root() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let root = dunce::canonicalize(dir.path()).expect("canonicalize temp dir");
    (dir, root)
}

/// Creates `entries` below `root`: `/` separates components, a trailing `/` makes a directory.
pub fn write_tree(root: &Path, entries: &[(&str, &[u8])]) {
    for (rel, content) in entries {
        let full = join_rel(root, rel.trim_end_matches('/'));
        let target = platform::fs_path(&full).expect("fs path");
        if rel.ends_with('/') {
            fs::create_dir_all(&target).expect("create dir");
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).expect("create parent");
            }
            fs::write(&target, content).expect("write file");
        }
    }
}

/// `root` joined with a `/`-separated relative path.
pub fn join_rel(root: &Path, rel: &str) -> PathBuf {
    rel.split('/')
        .filter(|part| !part.is_empty())
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

/// Every entry below `root`: folders map to None, files to their bytes, links to their target.
pub fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut out = BTreeMap::new();
    collect(root, Path::new(""), &mut out);
    out
}

fn collect(dir: &Path, rel: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
    let raw = platform::fs_path(dir).expect("fs path");
    for entry in fs::read_dir(raw).expect("list") {
        let entry = entry.expect("entry");
        let path = dir.join(entry.file_name());
        let rel_path = rel.join(entry.file_name());
        let kind = entry.file_type().expect("file type");
        if kind.is_symlink() {
            let target = fs::read_link(entry.path()).expect("read link");
            let text = format!("link:{}", target.display());
            out.insert(rel_path, Some(text.into_bytes()));
        } else if kind.is_dir() {
            out.insert(rel_path.clone(), None);
            collect(&path, &rel_path, out);
        } else {
            out.insert(rel_path, Some(fs::read(entry.path()).expect("read")));
        }
    }
}

/// A Paks folder with the three mod roots, and app folders, below one temp root.
pub struct FsFixture {
    pub dir: TempDir,
    pub root: PathBuf,
    pub paks: PathBuf,
    pub app: AppPaths,
}

pub fn fs_fixture() -> FsFixture {
    let (dir, root) = temp_root();
    let paks = root.join("Paks");
    write_tree(
        &paks,
        &[("Mods/", b""), ("~mods/", b""), ("LogicMods/", b"")],
    );
    let app = AppPaths::under(&root.join("app"));
    fs::create_dir_all(&app.data_dir).expect("data dir");
    fs::create_dir_all(&app.config_dir).expect("config dir");
    FsFixture {
        dir,
        root,
        paks,
        app,
    }
}

impl FsFixture {
    pub fn path(&self, rel: &str) -> PathBuf {
        join_rel(&self.paks, rel)
    }

    pub fn scope(&self) -> Scope {
        Scope::for_paks(&self.paks, &[&self.app.data_dir, &self.app.config_dir]).expect("scope")
    }

    pub fn gate(&self) -> OpGate {
        OpGate::open(&self.app).expect("open gate")
    }

    pub fn journal_path(&self) -> PathBuf {
        self.app.data_dir.join("journal.json")
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.app.data_dir.join("sessions")
    }

    /// A rename op between two paths below Paks.
    pub fn op(&self, from: &str, to: &str, is_dir: bool) -> RenameOp {
        RenameOp {
            from: self.path(from),
            to: self.path(to),
            is_dir,
        }
    }

    pub fn paks_snapshot(&self) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        snapshot(&self.paks)
    }
}

pub fn options(fault: Option<FaultPlan>) -> BatchOptions {
    BatchOptions {
        label: "test",
        retry: RetryMode::Never,
        fault,
    }
}

pub fn fail_before(index: usize) -> Option<FaultPlan> {
    Some(FaultPlan {
        fail_before: Some(index),
        ..FaultPlan::default()
    })
}

pub fn crash_after(done: usize) -> Option<FaultPlan> {
    Some(FaultPlan {
        crash_after: Some(done),
        ..FaultPlan::default()
    })
}

/// Names in `folder`, sorted, lossy (for assertions).
pub fn names_in(folder: &Path) -> Vec<String> {
    let raw = platform::fs_path(folder).expect("fs path");
    let mut names: Vec<String> = fs::read_dir(raw)
        .expect("list")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// The journal file parsed, or None when it does not exist.
pub fn journal_json(path: &Path) -> Option<serde_json::Value> {
    let bytes = fs::read(path).ok()?;
    Some(serde_json::from_slice(&bytes).expect("journal is JSON"))
}

/// Number of journal entries (0 when the file is missing).
pub fn journal_entries(path: &Path) -> usize {
    journal_json(path)
        .and_then(|json| json["entries"].as_array().map(Vec::len))
        .unwrap_or(0)
}

/// A trash env that only ever uses `<dir>/xdg-data/Trash` (the folder above it is created here,
/// as a real `$XDG_DATA_HOME` exists), never a real trash or mount topdir.
pub fn trash_env(dir: &Path) -> TrashEnv {
    let data = dir.join("xdg-data");
    fs::create_dir_all(platform::fs_path(&data).expect("fs path")).expect("xdg-data dir");
    TrashEnv {
        home_trash: Some(data.join("Trash")),
        uid: platform::current_uid(),
        topdirs: TopdirPolicy::Deny,
    }
}

/// Every entry below `root`, sorted: `d <rel>`, `f <rel> <size> <fnv-1a of the bytes>`,
/// `l <rel> -> <target>`. Equal listings mean equal names, kinds, sizes and bytes.
pub fn tree_listing(root: &Path) -> Vec<String> {
    snapshot(root)
        .into_iter()
        .map(|(rel, content)| {
            let rel = rel.to_string_lossy().replace('\\', "/");
            match content {
                None => format!("d {rel}"),
                Some(bytes) if bytes.starts_with(b"link:") => {
                    format!("l {rel} -> {}", String::from_utf8_lossy(&bytes[5..]))
                }
                Some(bytes) => format!("f {rel} {} {:016x}", bytes.len(), fnv1a(&bytes)),
            }
        })
        .collect()
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}
