//! Gate security: vertical boundary enforcement with I/O sharing contracts.
//!
//! A [`GateConfig`] belongs to a vault node and declares, per resource, which
//! directions of sharing that node permits:
//!
//! * **Expose (upward)** — the node makes a resource visible to its ancestors
//!   (e.g. a Chamber exposing a milestone up to the Counselor for oversight).
//! * **Accept (downward)** — the node accepts a resource pushed from an ancestor
//!   (e.g. the Master pushing a global template down into a Classroom).
//!
//! [`evaluate_access`] is the single choke point every cross-boundary read must
//! pass through. It is pure and total: given the actor's path, the target's
//! path, the requested resource, and the *target node's* gate, it returns
//! [`GateDecision::Allow`] or a structured [`GateDecision::Deny`]. Lateral
//! access (siblings / unrelated subtrees) is refused unconditionally.

use crate::vault::{Relationship, VaultPath};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A category of shareable data crossing a vault boundary. Modeled as an enum so
/// a typo can never silently widen a sharing policy (the framework doc's
/// "type-state / compile-time compliance" principle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    /// A student's core record (demographics, academics).
    StudentRecord,
    /// A milestone or achievement.
    Milestone,
    /// A stored document / file.
    Document,
    /// An essay draft (subject to version control).
    Essay,
    /// A billing / hour-ledger entry.
    BillingEntry,
    /// Context handed to an AI agent for grounding.
    AiContext,
    /// A reusable template or playbook.
    Template,
    /// An audit-log record.
    AuditRecord,
}

impl fmt::Display for ResourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// The direction a resource is allowed to flow across a boundary, expressed from
/// the owning node's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareDirection {
    /// Owner exposes the resource to its ancestors (child → parent visibility).
    Expose,
    /// Owner accepts the resource pushed from an ancestor (parent → child).
    Accept,
}

/// A single I/O sharing contract: "this node permits `resource` to flow
/// `direction`."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IoContract {
    /// The resource category the contract governs.
    pub resource: ResourceKind,
    /// The permitted direction of flow.
    pub direction: ShareDirection,
}

impl IoContract {
    /// Shorthand constructor.
    #[must_use]
    pub fn new(resource: ResourceKind, direction: ShareDirection) -> Self {
        Self {
            resource,
            direction,
        }
    }
}

/// The set of contracts a vault node exposes at its boundary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateConfig {
    contracts: Vec<IoContract>,
}

impl GateConfig {
    /// A sealed gate: no cross-boundary sharing at all (maximum isolation).
    #[must_use]
    pub fn sealed() -> Self {
        Self {
            contracts: Vec::new(),
        }
    }

    /// Build a gate from an explicit list of contracts.
    #[must_use]
    pub fn with_contracts(contracts: impl Into<Vec<IoContract>>) -> Self {
        Self {
            contracts: contracts.into(),
        }
    }

    /// Add a contract (builder style).
    #[must_use]
    pub fn allow(mut self, resource: ResourceKind, direction: ShareDirection) -> Self {
        self.contracts.push(IoContract::new(resource, direction));
        self
    }

    /// Whether this gate permits `resource` to flow in `direction`.
    #[must_use]
    pub fn permits(&self, resource: ResourceKind, direction: ShareDirection) -> bool {
        self.contracts
            .iter()
            .any(|c| c.resource == resource && c.direction == direction)
    }

    /// Read-only view of the configured contracts.
    #[must_use]
    pub fn contracts(&self) -> &[IoContract] {
        &self.contracts
    }
}

/// A request to read `resource` belonging to `target`, made by `actor`.
#[derive(Debug, Clone)]
pub struct AccessRequest<'a> {
    /// Path of the node making the request.
    pub actor: &'a VaultPath,
    /// Path of the node that owns the data being accessed.
    pub target: &'a VaultPath,
    /// The resource category being read.
    pub resource: ResourceKind,
}

/// The outcome of a gate evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDecision {
    /// The access is permitted.
    Allow,
    /// The access is refused, with a structured reason.
    Deny(DenyReason),
}

impl GateDecision {
    /// Convenience: was the access allowed?
    #[must_use]
    pub fn is_allowed(self) -> bool {
        matches!(self, GateDecision::Allow)
    }
}

/// Why a gate refused an access request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum DenyReason {
    /// The actor and target are on different subtrees (sibling / unrelated).
    /// Lateral access is never permitted.
    LateralIsolation,
    /// The relationship is vertical but the target's gate declares no contract
    /// permitting this resource in the required direction.
    NoContract {
        /// The resource that was requested.
        resource: ResourceKind,
        /// The direction the contract would have needed to allow.
        required: ShareDirection,
    },
}

impl fmt::Display for DenyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DenyReason::LateralIsolation => {
                write!(f, "lateral isolation: nodes are on different subtrees")
            }
            DenyReason::NoContract { resource, required } => {
                write!(f, "no contract for {resource} in direction {required:?}")
            }
        }
    }
}

/// Evaluate a cross-boundary access against the target node's gate.
///
/// This is the vertical-boundary choke point:
///
/// * **Same node** → always allowed (a node owns its data).
/// * **Actor is an ancestor** of the target → the actor is reaching *down* into
///   a descendant, so the target must `Expose` the resource upward.
/// * **Actor is a descendant** of the target → the actor is reaching *up* to an
///   ancestor, so the ancestor (target) must `Accept`/push the resource
///   downward.
/// * **Lateral** → denied unconditionally.
#[must_use]
pub fn evaluate_access(req: &AccessRequest<'_>, target_gate: &GateConfig) -> GateDecision {
    match req.actor.relationship_to(req.target) {
        Relationship::Same => GateDecision::Allow,
        Relationship::Lateral => GateDecision::Deny(DenyReason::LateralIsolation),
        Relationship::Ancestor => {
            // Actor sits above target; target must expose the resource upward.
            if target_gate.permits(req.resource, ShareDirection::Expose) {
                GateDecision::Allow
            } else {
                GateDecision::Deny(DenyReason::NoContract {
                    resource: req.resource,
                    required: ShareDirection::Expose,
                })
            }
        }
        Relationship::Descendant => {
            // Actor sits below target; target (ancestor) must share downward.
            if target_gate.permits(req.resource, ShareDirection::Accept) {
                GateDecision::Allow
            } else {
                GateDecision::Deny(DenyReason::NoContract {
                    resource: req.resource,
                    required: ShareDirection::Accept,
                })
            }
        }
    }
}
