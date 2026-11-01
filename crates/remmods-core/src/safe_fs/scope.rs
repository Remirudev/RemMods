//! Where RemMods may mutate: the three mod roots under Paks (renames, links, removals) and its
//! own app folders (JSON files). Every mutator checks its paths here first.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use super::FsError;
use crate::platform::{self, OsKind};

/// Names of the mod roots under Paks, matched ASCII-case-insensitively.
const MOD_ROOT_NAMES: [&str; 3] = ["Mods", "~mods", "LogicMods"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeViolation {
    NotAbsolute,
    DotComponent,
    OutsideRoots,
    IsRoot,
    LinkComponent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Root {
    lexical: PathBuf,
    canonical: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Scope {
    mod_roots: Vec<Root>,
    app_dirs: Vec<Root>,
}

impl Scope {
    /// Mod roots = every child of `paks` named `Mods`, `~mods` or `LogicMods` in any ASCII case;
    /// app dirs = `app_dirs`. Missing roots are skipped.
    pub fn for_paks(paks: &Path, app_dirs: &[&Path]) -> Result<Scope, FsError> {
        let listing = platform::fs_path(paks).and_then(fs::read_dir);
        let entries = match listing {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Self::build(&[], app_dirs.iter().copied()));
            }
            Err(error) => return Err(FsError::io("scope", paks, error)),
        };
        let mut mod_roots = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| FsError::io("scope", paks, error))?;
            let name = entry.file_name();
            let is_root_name = name.to_str().is_some_and(|name| {
                MOD_ROOT_NAMES
                    .iter()
                    .any(|root| root.eq_ignore_ascii_case(name))
            });
            if is_root_name {
                mod_roots.push(paks.join(name));
            }
        }
        Ok(Self::build(&mod_roots, app_dirs.iter().copied()))
    }

    /// Explicit roots (tests).
    pub fn from_roots(mod_roots: &[PathBuf], app_dirs: &[PathBuf]) -> Result<Scope, FsError> {
        Ok(Self::build(
            mod_roots,
            app_dirs.iter().map(PathBuf::as_path),
        ))
    }

    /// Mod paths: renames, links, removals, placeholders. App dirs do not count here.
    pub fn check(&self, path: &Path) -> Result<(), FsError> {
        ScopeChecker::new(self).check(path)
    }

    /// App-data files: the same walk against the app dirs only.
    pub fn check_app(&self, path: &Path) -> Result<(), FsError> {
        ScopeChecker::new(self).check_app(path)
    }

    /// True when `path` names one of the mod roots (lexically, or as the same real folder when
    /// `path` itself is no link).
    pub(crate) fn is_mod_root(&self, path: &Path) -> bool {
        self.mod_roots
            .iter()
            .any(|root| same_lexical(path, &root.lexical))
            || (!is_link(path)
                && canonicalize(path).is_some_and(|canonical| {
                    self.mod_roots
                        .iter()
                        .any(|root| root.canonical == canonical)
                }))
    }

    /// True when `path` (existing or not) is a mod root or an app dir or lies below one, judged
    /// on the lexical path and on the path with its existing part resolved.
    pub(crate) fn contains(&self, path: &Path) -> bool {
        let resolved = resolve_existing(path);
        self.mod_roots.iter().chain(&self.app_dirs).any(|root| {
            is_within(path, &root.lexical)
                || resolved
                    .as_deref()
                    .is_some_and(|resolved| is_within(resolved, &root.canonical))
        })
    }

    fn build<'a>(mod_roots: &[PathBuf], app_dirs: impl Iterator<Item = &'a Path>) -> Scope {
        Scope {
            mod_roots: mod_roots
                .iter()
                .filter_map(|root| existing_root(root))
                .collect(),
            app_dirs: app_dirs.filter_map(existing_root).collect(),
        }
    }
}

/// Scope checks with canonical paths cached, for the many paths of one batch.
pub(crate) struct ScopeChecker<'a> {
    scope: &'a Scope,
    canonical: HashMap<PathBuf, Option<PathBuf>>,
}

impl<'a> ScopeChecker<'a> {
    pub(crate) fn new(scope: &'a Scope) -> Self {
        Self {
            scope,
            canonical: HashMap::new(),
        }
    }

    pub(crate) fn check(&mut self, path: &Path) -> Result<(), FsError> {
        let roots = &self.scope.mod_roots;
        walk(path, roots, &mut self.canonical).map_err(|reason| out_of_scope(path, reason))
    }

    pub(crate) fn check_app(&mut self, path: &Path) -> Result<(), FsError> {
        let roots = &self.scope.app_dirs;
        walk(path, roots, &mut self.canonical).map_err(|reason| out_of_scope(path, reason))
    }
}

/// True when a component of `path` is exactly `.` or `..`, judged on the raw bytes because
/// `Path::components` drops inner `.` components.
pub(crate) fn has_dot_component(path: &Path) -> bool {
    path.as_os_str()
        .as_encoded_bytes()
        .split(|byte| byte.is_ascii() && std::path::is_separator(char::from(*byte)))
        .any(|part| part == b"." || part == b"..")
}

fn walk(
    path: &Path,
    roots: &[Root],
    cache: &mut HashMap<PathBuf, Option<PathBuf>>,
) -> Result<(), ScopeViolation> {
    if !path.is_absolute() {
        return Err(ScopeViolation::NotAbsolute);
    }
    if has_dot_component(path) {
        return Err(ScopeViolation::DotComponent);
    }
    let is_root_lexically = roots.iter().any(|root| same_lexical(path, &root.lexical));
    let mut is_root_canonically = || {
        !is_link(path)
            && canonical(path, cache)
                .is_some_and(|canonical| roots.iter().any(|root| root.canonical == canonical))
    };
    if is_root_lexically || is_root_canonically() {
        return Err(ScopeViolation::IsRoot);
    }
    for ancestor in path.ancestors().skip(1) {
        if roots
            .iter()
            .any(|root| same_lexical(ancestor, &root.lexical))
        {
            return Ok(());
        }
        // Before the canonical compare: a link inside a root that points at another root or an
        // app dir must not be accepted through its target.
        if is_link(ancestor) {
            return Err(ScopeViolation::LinkComponent);
        }
        if canonical(ancestor, cache)
            .is_some_and(|canonical| roots.iter().any(|root| root.canonical == canonical))
        {
            return Ok(());
        }
    }
    Err(ScopeViolation::OutsideRoots)
}

fn out_of_scope(path: &Path, reason: ScopeViolation) -> FsError {
    FsError::OutOfScope {
        path: path.to_path_buf(),
        reason,
    }
}

fn existing_root(root: &Path) -> Option<Root> {
    let canonical = canonicalize(root)?;
    Some(Root {
        lexical: root.to_path_buf(),
        canonical,
    })
}

fn canonical(path: &Path, cache: &mut HashMap<PathBuf, Option<PathBuf>>) -> Option<PathBuf> {
    cache
        .entry(path.to_path_buf())
        .or_insert_with(|| canonicalize(path))
        .clone()
}

fn canonicalize(path: &Path) -> Option<PathBuf> {
    platform::fs_path(path).and_then(fs::canonicalize).ok()
}

/// `path` with its longest existing ancestor canonicalised and the rest appended; None when no
/// ancestor resolves.
pub(crate) fn resolve_existing(path: &Path) -> Option<PathBuf> {
    let mut rest = Vec::new();
    for ancestor in path.ancestors() {
        if let Some(canonical) = canonicalize(ancestor) {
            return Some(
                rest.iter()
                    .rev()
                    .fold(canonical, |base, part| base.join(part)),
            );
        }
        rest.push(ancestor.file_name()?.to_os_string());
    }
    None
}

/// `path` equals `base` or lies below it, component-wise (ASCII case folded on Windows).
pub(crate) fn is_within(path: &Path, base: &Path) -> bool {
    let fold = platform::os_kind() == OsKind::Windows;
    let mut parts = path.components();
    base.components().all(|wanted| {
        parts
            .next()
            .is_some_and(|part| same_component(part, wanted, fold))
    })
}

/// Symlinks, and on Windows junctions (std reports name-surrogate reparse points as links).
fn is_link(path: &Path) -> bool {
    platform::fs_path(path)
        .and_then(fs::symlink_metadata)
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
}

/// Component-wise equality; ASCII-case-insensitive on Windows, where the file system folds case.
fn same_lexical(a: &Path, b: &Path) -> bool {
    let fold = platform::os_kind() == OsKind::Windows;
    let mut left = a.components();
    let mut right = b.components();
    loop {
        match (left.next(), right.next()) {
            (None, None) => return true,
            (Some(x), Some(y)) => {
                if !same_component(x, y, fold) {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

fn same_component(a: Component<'_>, b: Component<'_>, fold: bool) -> bool {
    let a = a.as_os_str().as_encoded_bytes();
    let b = b.as_os_str().as_encoded_bytes();
    if fold {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}
