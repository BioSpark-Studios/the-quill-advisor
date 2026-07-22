//! Plugin domain errors.

/// Errors from manifest (de)serialization, signing, and catalog operations.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// JSON (de)serialization failed.
    #[error("manifest serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// A hex-encoded key or signature was malformed.
    #[error("invalid hex encoding: {0}")]
    Hex(#[from] hex::FromHexError),

    /// A key or signature had the wrong length / was not a valid ed25519 value.
    #[error("invalid cryptographic material: {0}")]
    Crypto(String),

    /// A plugin id was already present in the catalog.
    #[error("plugin already installed: {0}")]
    AlreadyInstalled(String),

    /// A referenced plugin id was not found.
    #[error("plugin not found: {0}")]
    NotFound(String),
}

/// Plugin result alias.
pub type Result<T, E = PluginError> = std::result::Result<T, E>;
