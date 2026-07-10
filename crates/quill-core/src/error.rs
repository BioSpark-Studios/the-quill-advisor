//! Domain-specific error type for the Quill Advisor core.
//!
//! Adapted from the `NexusError` design in
//! `docs/architecture/quantum-nexus-core-framework.md`: a single typed error
//! enum (via `thiserror`) replaces scattered `String` errors so that security
//! and compliance boundaries are enforced at compile time rather than by
//! stringly-typed comparison.

use crate::gate::DenyReason;
use crate::vault::VaultLevel;

/// The canonical error type for all core domain operations.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CoreError {
    /// A gate boundary refused an access request. Carries the structured reason
    /// so callers (and the audit log) know *why* isolation held.
    #[error("vault gate denied access: {0}")]
    GateDenied(DenyReason),

    /// An operation targeted a vault level that is invalid for the request
    /// (e.g. creating a child under a `Chamber`, which is the deepest level).
    #[error("invalid vault level for operation: {0:?}")]
    InvalidLevel(VaultLevel),

    /// A vault path was malformed (empty, or its levels did not descend by
    /// exactly one at each step).
    #[error("malformed vault path: {0}")]
    MalformedPath(String),

    /// An AI agent was invoked in a chamber where it has not been authorized by
    /// the supervising counselor (per-chamber gate toggle).
    #[error("agent {agent} is not authorized in chamber {chamber}")]
    AgentNotAuthorized { agent: String, chamber: String },

    /// A compliance standard required by the operation was not satisfied.
    #[error("compliance violation: {0:?}")]
    ComplianceViolation(crate::compliance::ComplianceStandard),
}

/// Convenience alias mirroring the framework doc's `Result<T, E = NexusError>`.
pub type Result<T, E = CoreError> = std::result::Result<T, E>;
