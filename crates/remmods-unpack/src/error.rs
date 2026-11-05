use remmods_pak::PakError;

#[derive(Debug, thiserror::Error)]
pub enum UnpackError {
    #[error(transparent)]
    Toc(#[from] PakError),
    #[error("read failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("malformed container: {0}")]
    Malformed(&'static str),
    #[error("the chunk is too large to read")]
    TooLarge,
    #[error("the container is split into partitions")]
    Partitioned,
    #[error("unknown compression method")]
    UnknownMethod,
    #[error("a compressed block could not be read")]
    Decompress,
    #[error("the asset holds no readable texture")]
    NotATexture,
    #[error("unsupported texture format {0}")]
    UnsupportedFormat(String),
    #[error("the picture could not be encoded")]
    Encode,
    #[error("a decoder stopped on bad data")]
    Decoder,
    #[error("malformed item list: {0}")]
    ItemList(&'static str),
}
