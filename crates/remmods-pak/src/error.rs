#[derive(Debug, thiserror::Error)]
pub enum PakError {
    #[error("read failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("not an IoStore table of contents")]
    NotIoStore,
    #[error("unsupported TOC version {0}")]
    UnsupportedVersion(u8),
    #[error("the directory index is encrypted")]
    Encrypted,
    #[error("the container has no directory index")]
    NoDirectoryIndex,
    #[error("malformed TOC: {0}")]
    Malformed(&'static str),
    #[error("the directory index is too large")]
    TooLarge,
}
