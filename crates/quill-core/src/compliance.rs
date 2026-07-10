//! Compliance standards and the audit-trail record type.
//!
//! `VaultLevel` and `ComplianceStandard` are rigid enums (never strings) so that
//! Domain 4 (Security) boundaries cannot be breached by a typo — the
//! compile-time-compliance principle from the core framework doc.

use crate::gate::GateDecision;
use crate::ids::{ActorId, VaultId};
use crate::vault::VaultLevel;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Regulatory standards the platform enforces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComplianceStandard {
    /// U.S. Family Educational Rights and Privacy Act.
    Ferpa,
    /// EU General Data Protection Regulation.
    Gdpr,
    /// A named industry-specific standard.
    IndustrySpecific,
}

/// A single append-only audit record. The storage layer persists these to the
/// global `core_db`; the type lives in core so that gate evaluation and the
/// storage layer share one definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// When the event occurred (UTC).
    pub at: DateTime<Utc>,
    /// The human actor responsible, if any (system events may have none).
    pub actor: Option<ActorId>,
    /// The vault node the event concerns.
    pub vault: VaultId,
    /// A short machine-readable action code, e.g. `"gate.access"`.
    pub action: String,
    /// Free-form detail for the human reader.
    pub detail: String,
    /// Whether the underlying operation was allowed.
    pub allowed: bool,
}

impl AuditEntry {
    /// Build an audit entry describing the outcome of a gate evaluation.
    #[must_use]
    pub fn for_gate(
        actor: Option<ActorId>,
        vault: VaultId,
        decision: GateDecision,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            at: Utc::now(),
            actor,
            vault,
            action: "gate.access".to_string(),
            detail: detail.into(),
            allowed: decision.is_allowed(),
        }
    }
}

/// Marks a component that can assert its own regulatory compliance and emit an
/// audit trail (Domain 4 `Compliant` trait).
pub trait Compliant {
    /// Whether this component satisfies `standard`.
    fn verify_compliance(&self, standard: ComplianceStandard) -> bool;
    /// Produce the audit records this component has accumulated.
    fn audit_trail(&self) -> Vec<AuditEntry>;
}

/// Roles a human actor can hold, used for coarse RBAC checks at the domain
/// layer. Finer permissions live with the storage layer's RBAC tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Level-0 director with global oversight.
    Director,
    /// Level-1 counselor owning a workspace.
    Counselor,
    /// A student inside a chamber.
    Student,
    /// A parent with scoped, privacy-shielded visibility.
    Parent,
}

impl Role {
    /// The highest vault level this role may directly administer.
    #[must_use]
    pub const fn max_admin_level(self) -> VaultLevel {
        match self {
            Role::Director => VaultLevel::Master,
            Role::Counselor => VaultLevel::Counselor,
            Role::Student | Role::Parent => VaultLevel::Chamber,
        }
    }
}
