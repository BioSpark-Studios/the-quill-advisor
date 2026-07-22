//! Row/domain mapping types shared by the storage modules.

use chrono::{DateTime, Utc};
use quill_core::gate::GateConfig;
use quill_core::ids::VaultId;
use quill_core::vault::{VaultLevel, VaultNode};

/// A vault node as stored in `core_db.vault_directory`, enriched with the
/// physical `db_path` of its isolated store and its gate policy.
#[derive(Debug, Clone)]
pub struct VaultRecord {
    /// The pure-domain node (id, parent, level, name).
    pub node: VaultNode,
    /// Path to the isolated `vault_db` file, if this node roots one.
    pub db_path: Option<String>,
    /// This node's gate configuration.
    pub gate: GateConfig,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Convert a stored `level` integer (0..=3) into a [`VaultLevel`].
pub(crate) fn level_from_i64(level: i64) -> VaultLevel {
    match level {
        0 => VaultLevel::Master,
        1 => VaultLevel::Counselor,
        2 => VaultLevel::Classroom,
        _ => VaultLevel::Chamber,
    }
}

/// The integer discriminant persisted for a [`VaultLevel`].
pub(crate) fn level_to_i64(level: VaultLevel) -> i64 {
    i64::from(level.depth())
}

/// Parse a TEXT primary key back into a [`VaultId`].
pub(crate) fn parse_vault_id(s: &str) -> Result<VaultId, uuid::Error> {
    Ok(VaultId(uuid::Uuid::parse_str(s)?))
}
