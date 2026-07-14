use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{path}:{line}:{column}: {message}")]
    Syntax {
        path: String,
        line: usize,
        column: usize,
        message: String,
    },
    #[error("module not found: {0}")]
    ModuleNotFound(String),
    #[error("native module error: {0}")]
    Native(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn syntax(path: impl Into<String>, line: usize, column: usize, message: impl Into<String>) -> Self {
        Self::Syntax {
            path: path.into(),
            line,
            column,
            message: message.into(),
        }
    }
}
