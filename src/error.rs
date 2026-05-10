use std::io;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("pacman was not found in PATH")]
    PacmanNotFound,

    #[error("BlackArch repository groups were not found; ensure the BlackArch repo is configured")]
    BlackArchRepoNotFound,

    #[error("command `{command}` failed with status {status}: {stderr}")]
    CommandFailed {
        command: String,
        status: String,
        stderr: String,
    },

    #[error("invalid package name: {0}")]
    InvalidPackageName(String),

    #[error("invalid category name: {0}")]
    InvalidCategoryName(String),

    #[allow(dead_code)]
    #[error("failed to parse pacman output: {0}")]
    ParseError(String),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("config error: {0}")]
    Config(String),
}

impl From<toml::de::Error> for AppError {
    fn from(value: toml::de::Error) -> Self {
        Self::Config(value.to_string())
    }
}

impl From<toml::ser::Error> for AppError {
    fn from(value: toml::ser::Error) -> Self {
        Self::Config(value.to_string())
    }
}
