use thiserror::Error;

use crate::goodreads;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Goodreads error: {0}")]
    Goodreads(#[from] goodreads::GoodreadsError),
    #[error("book with id {0} already exists")]
    AlreadyExists(String),
    #[error("book with id {0} not found")]
    NotFound(String),
    #[error("invalid rating: {0} (must be 1-5)")]
    InvalidRating(u8),
    #[error("invalid rating value: '{0}'")]
    InvalidRatingValue(String),
    #[error("invalid status: '{0}' (use unread/reading/read/abandoned)")]
    InvalidStatus(String),
    #[error("'{0}' is neither a valid URL nor a valid ISBN; please pass a Goodreads URL or an ISBN")]
    InvalidAddQuery(String),
    #[error("HOME environment variable not set")]
    HomeNotFound,
}

pub type Result<T> = std::result::Result<T, AppError>;
