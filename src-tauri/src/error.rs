//! The one error type that crosses the IPC boundary (§12): already
//! user-presentable — the frontend shows it verbatim in the error alert.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A user-presentable error: title + message, §A copy.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[ts(export)]
pub struct AppError {
    /// Alert title.
    pub title: String,
    /// Alert body.
    pub message: String,
}

impl AppError {
    /// An error with the generic §A title.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            title: "Couldn’t complete that".to_string(),
            message: message.into(),
        }
    }

    /// An error with a specific title.
    pub fn titled(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.title, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::new(error.to_string())
    }
}

/// Command result alias.
pub type AppResult<T> = Result<T, AppError>;
