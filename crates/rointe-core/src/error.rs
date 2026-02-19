use thiserror::Error;

/// All errors that can be returned by `rointe-core`.
///
/// This enum is `#[non_exhaustive]` — new variants may be added in future
/// minor versions without a breaking change.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RointeError {
    /// Authentication failed (bad credentials, expired token, etc.).
    #[error("Authentication error: {0}")]
    Auth(String),

    /// An underlying HTTP transport error from `reqwest`.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// The Firebase Realtime Database returned an unexpected response or
    /// the response could not be parsed as the expected schema.
    #[error("Firebase error: {0}")]
    Firebase(String),

    /// A device ID was referenced that does not exist in the installation.
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    /// JSON serialisation or deserialisation failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Convenience `Result` alias for `rointe-core` operations.
pub type Result<T> = std::result::Result<T, RointeError>;
