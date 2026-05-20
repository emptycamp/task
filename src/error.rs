use std::fmt;

#[derive(Debug)]
pub enum Error {
    Db(heed::Error),
    Encode(bincode::Error),
    Yaml(serde_yml::Error),
    Io(std::io::Error),
    Parse(String),
    NotFound(u32),
    HistoryNotFound(u64),
    NothingToRevert,
    EditorError(String),
    Cancelled,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Db(e) => write!(f, "database error: {e}"),
            Self::Encode(e) => write!(f, "encoding error: {e}"),
            Self::Yaml(e) => write!(f, "yaml error: {e}"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
            Self::NotFound(id) => write!(f, "task #{id} not found"),
            Self::HistoryNotFound(id) => write!(f, "history event #{id} not found"),
            Self::NothingToRevert => write!(f, "nothing to revert"),
            Self::EditorError(msg) => write!(f, "editor error: {msg}"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::error::Error for Error {}

impl From<heed::Error> for Error {
    fn from(e: heed::Error) -> Self {
        Self::Db(e)
    }
}

impl From<bincode::Error> for Error {
    fn from(e: bincode::Error) -> Self {
        Self::Encode(e)
    }
}

impl From<serde_yml::Error> for Error {
    fn from(e: serde_yml::Error) -> Self {
        Self::Yaml(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
