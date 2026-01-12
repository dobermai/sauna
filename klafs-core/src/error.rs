use thiserror::Error;

/// Errors that can occur when interacting with the Klafs API
#[derive(Error, Debug)]
pub enum KlafsError {
    /// Authentication failed (wrong credentials or account locked)
    #[error("Authentication failed: {message}")]
    AuthenticationFailed { message: String },

    /// Account is locked after too many failed attempts
    #[error("Account locked: Klafs locks accounts after 3 failed login attempts. Please wait or contact support.")]
    AccountLocked,

    /// Session expired or invalid
    #[error("Session expired or invalid. Please login again.")]
    SessionExpired,

    /// Invalid PIN for power control
    #[error("Invalid PIN provided")]
    InvalidPin,

    /// Sauna not found
    #[error("Sauna with ID {sauna_id} not found")]
    SaunaNotFound { sauna_id: String },

    /// Sauna is not connected/reachable
    #[error("Sauna is not connected. Check your sauna's network connection.")]
    SaunaNotConnected,

    /// Invalid parameter value
    #[error("Invalid parameter: {message}")]
    InvalidParameter { message: String },

    /// API returned an error
    #[error("API error: {status_code} - {message}")]
    ApiError { status_code: u16, message: String },

    /// Network error
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON parsing error
    #[error("Failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),

    /// Failed to extract verification token
    #[error("Failed to extract verification token from response")]
    VerificationTokenNotFound,

    /// Unexpected response format
    #[error("Unexpected response: {message}")]
    UnexpectedResponse { message: String },
}

/// Result type for Klafs operations
pub type Result<T> = std::result::Result<T, KlafsError>;
