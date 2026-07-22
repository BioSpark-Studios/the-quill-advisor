//! # quill-core
//!
//! The pure domain layer for **The Quill Advisor**. No I/O, no async, no
//! framework types — just the data model and the rules that make the platform
//! trustworthy:
//!
//! * [`vault`] — the four-level `SSoT` hierarchy (Master → Counselor → Classroom →
//!   Chamber) and node relationships.
//! * [`gate`] — vertical boundary enforcement via I/O sharing contracts; the
//!   single [`gate::evaluate_access`] choke point every cross-boundary read
//!   passes through.
//! * [`authz`] — counselor-led per-chamber AI authorization.
//! * [`compliance`] — FERPA/GDPR standards, roles, and the audit-trail record.
//!
//! Everything here is unit-tested in isolation, which is the whole reason the
//! domain is a separate crate from storage and the Tauri shell.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod authz;
pub mod compliance;
pub mod error;
pub mod gate;
pub mod ids;
pub mod vault;

pub use error::{CoreError, Result};
pub use gate::{
    evaluate_access, AccessRequest, DenyReason, GateConfig, GateDecision, IoContract, ResourceKind,
    ShareDirection,
};
pub use ids::{ActorId, AgentId, VaultId};
pub use vault::{Relationship, VaultLevel, VaultNode, VaultPath};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authz::ChamberAiAuthorization;

    /// Build a four-level path Master→Counselor→Classroom→Chamber sharing the
    /// given ancestor ids so relationships are deterministic.
    fn paths() -> (VaultId, VaultId, VaultId, VaultId) {
        (
            VaultId::new(),
            VaultId::new(),
            VaultId::new(),
            VaultId::new(),
        )
    }

    #[test]
    fn level_ordering_and_depth() {
        assert!(VaultLevel::Master < VaultLevel::Chamber);
        assert_eq!(VaultLevel::Master.depth(), 0);
        assert_eq!(VaultLevel::Chamber.depth(), 3);
        assert_eq!(
            VaultLevel::Master.child_level(),
            Some(VaultLevel::Counselor)
        );
        assert_eq!(VaultLevel::Chamber.child_level(), None);
        assert_eq!(
            VaultLevel::Chamber.parent_level(),
            Some(VaultLevel::Classroom)
        );
    }

    #[test]
    fn child_of_chamber_is_rejected() {
        let m = VaultNode::master(VaultId::new(), "Master");
        let c = VaultNode::child_of(&m, VaultId::new(), "Counselor A").unwrap();
        let room = VaultNode::child_of(&c, VaultId::new(), "Room 1").unwrap();
        let chamber = VaultNode::child_of(&room, VaultId::new(), "Student S").unwrap();
        assert_eq!(chamber.level, VaultLevel::Chamber);
        // A chamber has no children.
        let err = VaultNode::child_of(&chamber, VaultId::new(), "nope").unwrap_err();
        assert_eq!(err, CoreError::InvalidLevel(VaultLevel::Chamber));
    }

    #[test]
    fn path_level_and_relationships() {
        let (m, c, r, s) = paths();
        let master = VaultPath::new(vec![m]).unwrap();
        let counselor = VaultPath::new(vec![m, c]).unwrap();
        let chamber = VaultPath::new(vec![m, c, r, s]).unwrap();

        assert_eq!(master.level(), VaultLevel::Master);
        assert_eq!(chamber.level(), VaultLevel::Chamber);

        assert_eq!(master.relationship_to(&chamber), Relationship::Ancestor);
        assert_eq!(chamber.relationship_to(&master), Relationship::Descendant);
        assert_eq!(counselor.relationship_to(&counselor), Relationship::Same);
    }

    #[test]
    fn siblings_are_laterally_isolated() {
        let (m, c, r, _) = paths();
        let s1 = VaultId::new();
        let s2 = VaultId::new();
        let chamber_a = VaultPath::new(vec![m, c, r, s1]).unwrap();
        let chamber_b = VaultPath::new(vec![m, c, r, s2]).unwrap();
        assert_eq!(chamber_a.relationship_to(&chamber_b), Relationship::Lateral);

        // Even a fully open gate cannot enable lateral access.
        let open = GateConfig::sealed()
            .allow(ResourceKind::StudentRecord, ShareDirection::Expose)
            .allow(ResourceKind::StudentRecord, ShareDirection::Accept);
        let req = AccessRequest {
            actor: &chamber_a,
            target: &chamber_b,
            resource: ResourceKind::StudentRecord,
        };
        assert_eq!(
            evaluate_access(&req, &open),
            GateDecision::Deny(DenyReason::LateralIsolation)
        );
    }

    #[test]
    fn upward_read_requires_expose_contract() {
        let (m, c, r, s) = paths();
        let counselor = VaultPath::new(vec![m, c]).unwrap();
        let chamber = VaultPath::new(vec![m, c, r, s]).unwrap();

        // Counselor (ancestor) tries to read the chamber's student record.
        let req = AccessRequest {
            actor: &counselor,
            target: &chamber,
            resource: ResourceKind::StudentRecord,
        };

        // Sealed chamber → denied.
        assert_eq!(
            evaluate_access(&req, &GateConfig::sealed()),
            GateDecision::Deny(DenyReason::NoContract {
                resource: ResourceKind::StudentRecord,
                required: ShareDirection::Expose,
            })
        );

        // Chamber exposes StudentRecord upward → allowed.
        let gate = GateConfig::sealed().allow(ResourceKind::StudentRecord, ShareDirection::Expose);
        assert_eq!(evaluate_access(&req, &gate), GateDecision::Allow);

        // But exposing only Milestones does not leak the StudentRecord.
        let narrow = GateConfig::sealed().allow(ResourceKind::Milestone, ShareDirection::Expose);
        assert!(!evaluate_access(&req, &narrow).is_allowed());
    }

    #[test]
    fn downward_read_requires_accept_contract() {
        let (m, c, r, s) = paths();
        let master = VaultPath::new(vec![m]).unwrap();
        let chamber = VaultPath::new(vec![m, c, r, s]).unwrap();

        // Chamber (descendant) tries to pull a Template from the Master (ancestor).
        let req = AccessRequest {
            actor: &chamber,
            target: &master,
            resource: ResourceKind::Template,
        };
        assert!(!evaluate_access(&req, &GateConfig::sealed()).is_allowed());

        let gate = GateConfig::sealed().allow(ResourceKind::Template, ShareDirection::Accept);
        assert_eq!(evaluate_access(&req, &gate), GateDecision::Allow);
    }

    #[test]
    fn own_data_is_always_readable() {
        let (m, c, _, _) = paths();
        let counselor = VaultPath::new(vec![m, c]).unwrap();
        let req = AccessRequest {
            actor: &counselor,
            target: &counselor,
            resource: ResourceKind::BillingEntry,
        };
        assert_eq!(
            evaluate_access(&req, &GateConfig::sealed()),
            GateDecision::Allow
        );
    }

    #[test]
    fn chamber_ai_authorization_toggles() {
        let chamber = VaultId::new();
        let quill = AgentId::new();
        let other = AgentId::new();
        let mut auth = ChamberAiAuthorization::disabled(chamber);

        // Disabled by default.
        assert!(!auth.is_authorized(quill));
        assert!(auth.ensure_authorized(quill).is_err());

        // Allow-listing an agent while the master switch is off is still denied.
        auth.authorize(quill);
        assert!(!auth.is_authorized(quill));

        // Enable the chamber → the allow-listed agent runs, others don't.
        auth.set_enabled(true);
        assert!(auth.is_authorized(quill));
        assert!(!auth.is_authorized(other));
        assert!(auth.ensure_authorized(quill).is_ok());

        // Revoking removes access even while enabled.
        auth.revoke(quill);
        assert!(!auth.is_authorized(quill));
    }

    #[test]
    fn gate_config_serde_roundtrips() {
        let gate = GateConfig::sealed()
            .allow(ResourceKind::Essay, ShareDirection::Expose)
            .allow(ResourceKind::Template, ShareDirection::Accept);
        let json = serde_json::to_string(&gate).unwrap();
        let back: GateConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(gate, back);
    }
}
