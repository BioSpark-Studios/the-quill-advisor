//! The `SSoT` vault hierarchy: levels, nodes, paths, and relationships.
//!
//! Every doc agrees on a four-level Single Source of Truth structure with
//! sandboxed portals:
//!
//! * **Level 0 — Master**: director control / global settings.
//! * **Level 1 — Counselor**: a counselor's private workspace.
//! * **Level 2 — Classroom**: a sub-vault / cohort / meeting room.
//! * **Level 3 — Chamber**: an individual student's sandbox.
//!
//! Isolation is *vertical*: data may only cross a level boundary through an
//! explicit gate contract (see [`crate::gate`]). Siblings are invisible to each
//! other. This module provides the pure structural facts (who is an ancestor of
//! whom); [`crate::gate`] layers the sharing policy on top.

use crate::error::{CoreError, Result};
use crate::ids::VaultId;
use serde::{Deserialize, Serialize};

/// The four hierarchy levels, ordered from most-privileged (root) to most-isolated (leaf).
///
/// The discriminants matter: `Master = 0 .. Chamber = 3` equals the node's depth
/// from the root, and the derived `Ord` means `Master < Counselor < ... < Chamber`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultLevel {
    /// Level 0 — the global Master Vault (director control).
    Master = 0,
    /// Level 1 — a Counselor's regular vault / workspace.
    Counselor = 1,
    /// Level 2 — a Classroom / sub-vault.
    Classroom = 2,
    /// Level 3 — a student Chamber (leaf sandbox).
    Chamber = 3,
}

impl VaultLevel {
    /// Depth from the root (Master = 0).
    #[must_use]
    pub const fn depth(self) -> u8 {
        self as u8
    }

    /// The level directly beneath this one, or `None` for a `Chamber` (the leaf).
    #[must_use]
    pub const fn child_level(self) -> Option<VaultLevel> {
        match self {
            VaultLevel::Master => Some(VaultLevel::Counselor),
            VaultLevel::Counselor => Some(VaultLevel::Classroom),
            VaultLevel::Classroom => Some(VaultLevel::Chamber),
            VaultLevel::Chamber => None,
        }
    }

    /// The level directly above this one, or `None` for `Master` (the root).
    #[must_use]
    pub const fn parent_level(self) -> Option<VaultLevel> {
        match self {
            VaultLevel::Master => None,
            VaultLevel::Counselor => Some(VaultLevel::Master),
            VaultLevel::Classroom => Some(VaultLevel::Counselor),
            VaultLevel::Chamber => Some(VaultLevel::Classroom),
        }
    }

    /// Whether a node at this level may own an isolated `vault_db` file.
    ///
    /// Master lives in the shared `core_db`; every Counselor-level vault and
    /// below is a candidate for physical file isolation, but in practice the
    /// storage layer materializes one `vault_db` per Counselor vault and nests
    /// classrooms/chambers inside it.
    #[must_use]
    pub const fn is_isolatable(self) -> bool {
        !matches!(self, VaultLevel::Master)
    }
}

/// Metadata describing a single node in the hierarchy. Pure data — no I/O.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultNode {
    /// Stable identifier for this node.
    pub id: VaultId,
    /// The parent node, or `None` for the Master root.
    pub parent: Option<VaultId>,
    /// This node's level in the hierarchy.
    pub level: VaultLevel,
    /// Human-readable name shown on the vault card.
    pub name: String,
}

impl VaultNode {
    /// Construct the singleton Master root.
    #[must_use]
    pub fn master(id: VaultId, name: impl Into<String>) -> Self {
        Self {
            id,
            parent: None,
            level: VaultLevel::Master,
            name: name.into(),
        }
    }

    /// Construct a child node beneath `parent`, deriving the child level.
    ///
    /// # Errors
    /// Returns [`CoreError::InvalidLevel`] if `parent` is a `Chamber` (no children allowed).
    pub fn child_of(parent: &VaultNode, id: VaultId, name: impl Into<String>) -> Result<Self> {
        let level = parent
            .level
            .child_level()
            .ok_or(CoreError::InvalidLevel(parent.level))?;
        Ok(Self {
            id,
            parent: Some(parent.id),
            level,
            name: name.into(),
        })
    }
}

/// A root-to-node path expressed as the ordered list of ancestor ids ending
/// with the node itself. Path length equals `level.depth() + 1`.
///
/// Relationships between two nodes are determined purely by prefix comparison
/// of their paths, which is what makes gate evaluation testable without a live
/// tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultPath(Vec<VaultId>);

impl VaultPath {
    /// Build a path from an ordered slice of ids (root first, node last).
    ///
    /// # Errors
    /// Returns [`CoreError::MalformedPath`] if the slice is empty or too deep to
    /// correspond to a valid `Master..=Chamber` chain.
    pub fn new(ids: impl Into<Vec<VaultId>>) -> Result<Self> {
        let ids = ids.into();
        if ids.is_empty() {
            return Err(CoreError::MalformedPath("path is empty".into()));
        }
        if ids.len() > 4 {
            return Err(CoreError::MalformedPath(format!(
                "path depth {} exceeds Chamber (max 4 nodes)",
                ids.len()
            )));
        }
        Ok(Self(ids))
    }

    /// The id of the node this path points at (the leaf of the path).
    ///
    /// # Panics
    /// Never in practice: [`VaultPath::new`] rejects empty paths, so the inner
    /// vector always has a last element.
    #[must_use]
    pub fn leaf(&self) -> VaultId {
        *self
            .0
            .last()
            .expect("VaultPath is never empty by construction")
    }

    /// The level of the node this path points at, derived from its depth.
    #[must_use]
    pub fn level(&self) -> VaultLevel {
        match self.0.len() {
            1 => VaultLevel::Master,
            2 => VaultLevel::Counselor,
            3 => VaultLevel::Classroom,
            _ => VaultLevel::Chamber,
        }
    }

    /// The ordered ids, root first.
    #[must_use]
    pub fn ids(&self) -> &[VaultId] {
        &self.0
    }

    /// Determine this node's relationship to `other`, using only path prefixes.
    #[must_use]
    pub fn relationship_to(&self, other: &VaultPath) -> Relationship {
        if self.0 == other.0 {
            return Relationship::Same;
        }
        if is_strict_prefix(&self.0, &other.0) {
            // self is above other → self is an ancestor of other.
            return Relationship::Ancestor;
        }
        if is_strict_prefix(&other.0, &self.0) {
            // other is above self → self is a descendant of other.
            return Relationship::Descendant;
        }
        Relationship::Lateral
    }
}

/// How one node relates to another in the hierarchy, from the first node's
/// point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relationship {
    /// The two paths point at the same node.
    Same,
    /// This node is an ancestor of the other (sits above it).
    Ancestor,
    /// This node is a descendant of the other (sits below it).
    Descendant,
    /// Neither is an ancestor of the other — siblings or unrelated subtrees.
    /// Lateral access is always denied (the sandbox boundary).
    Lateral,
}

/// True when `short` is a proper prefix of `long` (strictly shorter, all elements match).
fn is_strict_prefix(short: &[VaultId], long: &[VaultId]) -> bool {
    short.len() < long.len() && long.starts_with(short)
}
