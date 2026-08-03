use thiserror::Error;

#[derive(Debug, Error)]
pub enum GetinbedError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unknown format: {0}")]
    UnknownFormat(String),
    #[error("Column index {0} out of range (file has {1} columns)")]
    ColumnOutOfRange(usize, usize),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("{0}")]
    Other(String),
}
