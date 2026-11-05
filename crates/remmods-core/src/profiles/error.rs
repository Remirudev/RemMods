use crate::safe_fs::FsError;

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error(transparent)]
    Fs(#[from] FsError),
    #[error("profile not found")]
    NotFound,
    #[error("profile name is empty or too long")]
    InvalidName,
    #[error("a profile with this name exists")]
    NameTaken,
    #[error("the picked mod list has an empty, overlong or repeated key")]
    InvalidEntries,
    #[error("profiles.json is from a newer RemMods")]
    ReadOnly,
    #[error("the file is not a profile list ({0:?})")]
    Unparsable(ImportProblem),
    #[error("the diff changed since it was shown")]
    DiffChanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportProblem {
    TooLarge,
    NotJson,
    WrongShape,
    WrongFormat,
    NewerVersion,
}

impl ProfileError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Fs(_) => "fs",
            Self::NotFound => "not_found",
            Self::InvalidName => "invalid_name",
            Self::NameTaken => "name_taken",
            Self::InvalidEntries => "invalid_entries",
            Self::ReadOnly => "read_only",
            Self::Unparsable(_) => "unparsable",
            Self::DiffChanged => "diff_changed",
        }
    }
}
