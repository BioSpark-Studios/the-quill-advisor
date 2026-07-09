//! AI-layer error type.

use quill_core::CoreError;

/// Errors from the Omni-Route router and agents.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    /// The HTTP request to a provider failed.
    #[error("provider request failed: {0}")]
    Http(String),

    /// The provider returned a non-success status.
    #[error("provider {provider} returned status {status}: {body}")]
    Status {
        /// Which provider responded.
        provider: String,
        /// HTTP status code.
        status: u16,
        /// Response body (truncated).
        body: String,
    },

    /// The provider's response could not be parsed into a completion.
    #[error("could not parse {provider} response: {detail}")]
    Parse {
        /// Which provider responded.
        provider: String,
        /// What went wrong.
        detail: String,
    },

    /// No API key was configured for a provider that requires one.
    #[error("no API key configured for provider {0}")]
    MissingKey(String),

    /// Every provider in the router failed.
    #[error("all providers exhausted; last error: {0}")]
    AllProvidersFailed(String),

    /// The router has no providers configured.
    #[error("router has no providers configured")]
    NoProviders,

    /// A domain rule (e.g. per-chamber AI authorization) was violated.
    #[error(transparent)]
    Core(#[from] CoreError),
}

/// AI result alias.
pub type Result<T, E = AiError> = std::result::Result<T, E>;
