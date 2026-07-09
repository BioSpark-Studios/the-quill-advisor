//! The global `core_db` (Level-0 Master store).
//!
//! Opens SQLite in **WAL** mode, applies the embedded `core` migrations, and
//! exposes typed operations over the vault directory, audit log, and per-chamber
//! AI authorization. Uses SQLx's runtime query API (no compile-time DB needed).

use crate::error::{Result, StorageError};
use crate::models::{level_from_i64, level_to_i64, parse_vault_id, VaultRecord};
use chrono::{DateTime, Utc};
use quill_core::authz::ChamberAiAuthorization;
use quill_core::compliance::AuditEntry;
use quill_core::gate::GateConfig;
use quill_core::ids::VaultId;
use quill_core::vault::{VaultNode, VaultPath};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;

static CORE_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/core");

/// Handle to the global core database.
#[derive(Clone)]
pub struct CoreDb {
    pool: SqlitePool,
}

impl CoreDb {
    /// Open (creating if absent) the core_db at `path` in WAL mode and migrate it.
    ///
    /// # Errors
    /// Fails if the file cannot be opened or a migration errors.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let opts = SqliteConnectOptions::new()
            .filename(path.as_ref())
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);
        Self::from_options(opts).await
    }

    /// Open an isolated in-memory core_db (used by tests). Each pool connection
    /// shares one in-memory database.
    ///
    /// # Errors
    /// Fails if the in-memory database cannot be created or migrated.
    pub async fn open_in_memory() -> Result<Self> {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        // A single connection keeps the in-memory DB alive for the pool's life.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        CORE_MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    async fn from_options(opts: SqliteConnectOptions) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await?;
        CORE_MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    /// The underlying pool, for advanced callers.
    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // --- vault directory -----------------------------------------------------

    /// Ensure a Master (Level 0) node exists, returning it. Idempotent.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn ensure_master(&self, name: &str) -> Result<VaultRecord> {
        if let Some(rec) = self.master().await? {
            return Ok(rec);
        }
        let master = VaultNode::master(VaultId::new(), name);
        self.insert_node(&master, None, &GateConfig::sealed())
            .await?;
        self.get_node(master.id)
            .await?
            .ok_or_else(|| StorageError::NotFound("master just inserted".into()))
    }

    /// The Master node, if one exists.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn master(&self) -> Result<Option<VaultRecord>> {
        let row = sqlx::query("SELECT * FROM vault_directory WHERE level = 0 LIMIT 1")
            .fetch_optional(&self.pool)
            .await?;
        row.map(|r| self.row_to_record(&r)).transpose()
    }

    /// Insert a node into the directory.
    ///
    /// # Errors
    /// Propagates database and serialization errors.
    pub async fn insert_node(
        &self,
        node: &VaultNode,
        db_path: Option<&str>,
        gate: &GateConfig,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO vault_directory (id, parent_id, level, name, db_path, gate_config, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(node.id.to_string())
        .bind(node.parent.map(|p| p.to_string()))
        .bind(level_to_i64(node.level))
        .bind(&node.name)
        .bind(db_path)
        .bind(serde_json::to_string(gate)?)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Fetch a single node by id.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn get_node(&self, id: VaultId) -> Result<Option<VaultRecord>> {
        let row = sqlx::query("SELECT * FROM vault_directory WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(|r| self.row_to_record(&r)).transpose()
    }

    /// Direct children of `parent`.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn children(&self, parent: VaultId) -> Result<Vec<VaultRecord>> {
        let rows =
            sqlx::query("SELECT * FROM vault_directory WHERE parent_id = ? ORDER BY created_at")
                .bind(parent.to_string())
                .fetch_all(&self.pool)
                .await?;
        rows.iter().map(|r| self.row_to_record(r)).collect()
    }

    /// Every node in the directory.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn all_nodes(&self) -> Result<Vec<VaultRecord>> {
        let rows = sqlx::query("SELECT * FROM vault_directory ORDER BY level, created_at")
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(|r| self.row_to_record(r)).collect()
    }

    /// Build the root-to-node [`VaultPath`] for `id` by walking parent links.
    ///
    /// # Errors
    /// Returns [`StorageError::NotFound`] if the node (or a broken ancestor link)
    /// is missing.
    pub async fn path_of(&self, id: VaultId) -> Result<VaultPath> {
        let mut chain = Vec::new();
        let mut cursor = Some(id);
        while let Some(current) = cursor {
            let rec = self
                .get_node(current)
                .await?
                .ok_or_else(|| StorageError::NotFound(format!("vault node {current}")))?;
            chain.push(current);
            cursor = rec.node.parent;
        }
        chain.reverse();
        VaultPath::new(chain).map_err(StorageError::Core)
    }

    /// Persist a new gate configuration for a node.
    ///
    /// # Errors
    /// Propagates database and serialization errors.
    pub async fn set_gate(&self, id: VaultId, gate: &GateConfig) -> Result<()> {
        sqlx::query("UPDATE vault_directory SET gate_config = ? WHERE id = ?")
            .bind(serde_json::to_string(gate)?)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- settings ------------------------------------------------------------

    /// Read a global setting by key.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get::<String, _>("value")))
    }

    /// Insert or update a global setting.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn upsert_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // --- audit log -----------------------------------------------------------

    /// Append an audit entry.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn append_audit(&self, entry: &AuditEntry) -> Result<()> {
        sqlx::query(
            "INSERT INTO audit_log (at, actor_id, vault_id, action, detail, allowed)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(entry.at.to_rfc3339())
        .bind(entry.actor.map(|a| a.to_string()))
        .bind(entry.vault.to_string())
        .bind(&entry.action)
        .bind(&entry.detail)
        .bind(i64::from(entry.allowed))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Count audit entries (handy for assertions/monitoring).
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn audit_count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) AS n FROM audit_log")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("n"))
    }

    // --- AI authorization ----------------------------------------------------

    /// Load a chamber's AI authorization, defaulting to disabled if unset.
    ///
    /// # Errors
    /// Propagates database and deserialization errors.
    pub async fn ai_authorization(&self, chamber: VaultId) -> Result<ChamberAiAuthorization> {
        let row = sqlx::query(
            "SELECT enabled, authorized_agents FROM ai_authorization WHERE chamber_id = ?",
        )
        .bind(chamber.to_string())
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(ChamberAiAuthorization::disabled(chamber));
        };
        let mut auth = ChamberAiAuthorization::disabled(chamber);
        auth.set_enabled(row.get::<i64, _>("enabled") != 0);
        let agents: Vec<String> = serde_json::from_str(&row.get::<String, _>("authorized_agents"))?;
        for a in agents {
            if let Ok(uuid) = uuid::Uuid::parse_str(&a) {
                auth.authorize(quill_core::ids::AgentId(uuid));
            }
        }
        Ok(auth)
    }

    /// Persist a chamber's AI authorization state.
    ///
    /// # Errors
    /// Propagates database and serialization errors.
    pub async fn set_ai_authorization(&self, auth: &ChamberAiAuthorization) -> Result<()> {
        let agents: Vec<String> = auth
            .authorized_agents()
            .iter()
            .map(ToString::to_string)
            .collect();
        sqlx::query(
            "INSERT INTO ai_authorization (chamber_id, enabled, authorized_agents)
             VALUES (?, ?, ?)
             ON CONFLICT(chamber_id) DO UPDATE SET enabled = excluded.enabled,
                                                   authorized_agents = excluded.authorized_agents",
        )
        .bind(auth.chamber.to_string())
        .bind(i64::from(auth.enabled))
        .bind(serde_json::to_string(&agents)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // --- helpers -------------------------------------------------------------

    fn row_to_record(&self, row: &sqlx::sqlite::SqliteRow) -> Result<VaultRecord> {
        let id = parse_vault_id(row.get::<String, _>("id").as_str())
            .map_err(|e| StorageError::InvalidVaultPath(e.to_string()))?;
        let parent = row
            .get::<Option<String>, _>("parent_id")
            .map(|p| parse_vault_id(&p))
            .transpose()
            .map_err(|e| StorageError::InvalidVaultPath(e.to_string()))?;
        let level = level_from_i64(row.get::<i64, _>("level"));
        let name: String = row.get("name");
        let gate: GateConfig = serde_json::from_str(&row.get::<String, _>("gate_config"))?;
        let created_at = DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        Ok(VaultRecord {
            node: VaultNode {
                id,
                parent,
                level,
                name,
            },
            db_path: row.get("db_path"),
            gate,
            created_at,
        })
    }
}
