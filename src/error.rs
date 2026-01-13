//! Error types for FOSSA API operations.

use thiserror::Error;

/// Errors that can occur during FOSSA API operations.
#[derive(Debug, Error)]
pub enum FossaError {
    /// Configuration is missing or incomplete.
    #[error("FOSSA configuration required: {0}")]
    ConfigMissing(String),

    /// Invalid locator format.
    #[error("Invalid locator '{0}': expected format like 'custom+org/project$revision'")]
    InvalidLocator(String),

    /// Entity not found.
    #[error("{entity_type} '{id}' not found")]
    NotFound {
        entity_type: &'static str,
        id: String,
    },

    /// API request failed.
    #[error("FOSSA API error: {message}")]
    ApiError {
        message: String,
        status_code: Option<u16>,
    },

    /// HTTP transport error.
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// JSON parsing error.
    #[error("Failed to parse response: {0}")]
    ParseError(#[from] serde_json::Error),

    /// URL parsing error.
    #[error("Invalid URL: {0}")]
    UrlError(#[from] url::ParseError),

    /// Rate limited.
    #[error("Rate limited, retry after {retry_after_secs:?} seconds")]
    RateLimited { retry_after_secs: Option<u64> },
}

/// Result type alias for FOSSA operations.
pub type Result<T> = core::result::Result<T, FossaError>;
