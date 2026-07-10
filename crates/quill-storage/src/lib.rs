//! # quill-storage
//!
//! Persistence for The Quill Advisor, split to mirror the data-isolation model:
//!
//! * [`core_db::CoreDb`] — the single global Level-0 store (vault directory,
//!   audit log, licensing, RBAC, per-chamber AI authorization).
//! * [`vault_db::VaultDb`] — one physically isolated SQLite file per Counselor
//!   vault; the file boundary is the privacy boundary. Includes `.qavault`
//!   export/import.
//!
//! Both open SQLite in WAL mode and apply embedded migrations. The crate uses
//! SQLx's runtime query API, so it builds and tests without a live database.

#![forbid(unsafe_code)]
#![warn(clippy::all)]

pub mod core_db;
pub mod error;
pub mod models;
pub mod vault_db;
pub mod vault_manager;

pub use core_db::CoreDb;
pub use error::{Result, StorageError};
pub use models::VaultRecord;
pub use vault_db::{EssayVersion, Milestone, QaVault, Student, VaultDb};
pub use vault_manager::VaultManager;

#[cfg(test)]
mod tests {
    use super::*;
    use quill_core::gate::{GateConfig, ResourceKind, ShareDirection};
    use quill_core::vault::{VaultLevel, VaultNode};

    #[tokio::test]
    async fn core_db_builds_hierarchy_and_paths() {
        let db = CoreDb::open_in_memory().await.unwrap();
        let master = db.ensure_master("Summit Master").await.unwrap();
        assert_eq!(master.node.level, VaultLevel::Master);

        // ensure_master is idempotent.
        let again = db.ensure_master("ignored").await.unwrap();
        assert_eq!(master.node.id, again.node.id);

        let counselor =
            VaultNode::child_of(&master.node, quill_core::VaultId::new(), "Ms. Rivera").unwrap();
        db.insert_node(&counselor, Some("/vaults/rivera.db"), &GateConfig::sealed())
            .await
            .unwrap();
        let room =
            VaultNode::child_of(&counselor, quill_core::VaultId::new(), "Class of 2027").unwrap();
        db.insert_node(&room, None, &GateConfig::sealed())
            .await
            .unwrap();
        let chamber = VaultNode::child_of(&room, quill_core::VaultId::new(), "Alex").unwrap();
        db.insert_node(&chamber, None, &GateConfig::sealed())
            .await
            .unwrap();

        // Path walks all the way to the Master root.
        let path = db.path_of(chamber.id).await.unwrap();
        assert_eq!(path.level(), VaultLevel::Chamber);
        assert_eq!(path.ids().len(), 4);
        assert_eq!(path.ids()[0], master.node.id);

        assert_eq!(db.children(master.node.id).await.unwrap().len(), 1);
        assert_eq!(db.all_nodes().await.unwrap().len(), 4);
    }

    #[tokio::test]
    async fn gate_config_persists_across_reload() {
        let db = CoreDb::open_in_memory().await.unwrap();
        let master = db.ensure_master("M").await.unwrap();
        let gate = GateConfig::sealed().allow(ResourceKind::Milestone, ShareDirection::Expose);
        db.set_gate(master.node.id, &gate).await.unwrap();
        let reloaded = db.get_node(master.node.id).await.unwrap().unwrap();
        assert!(reloaded
            .gate
            .permits(ResourceKind::Milestone, ShareDirection::Expose));
        assert!(!reloaded
            .gate
            .permits(ResourceKind::StudentRecord, ShareDirection::Expose));
    }

    #[tokio::test]
    async fn ai_authorization_roundtrips() {
        let db = CoreDb::open_in_memory().await.unwrap();
        let chamber = quill_core::VaultId::new();
        let agent = quill_core::AgentId::new();

        // Defaults to disabled when never set.
        assert!(!db.ai_authorization(chamber).await.unwrap().enabled);

        let mut auth = quill_core::authz::ChamberAiAuthorization::disabled(chamber);
        auth.set_enabled(true);
        auth.authorize(agent);
        db.set_ai_authorization(&auth).await.unwrap();

        let loaded = db.ai_authorization(chamber).await.unwrap();
        assert!(loaded.is_authorized(agent));
    }

    #[tokio::test]
    async fn settings_upsert_and_read() {
        let db = CoreDb::open_in_memory().await.unwrap();
        assert!(db.get_setting("card:v1").await.unwrap().is_none());
        db.upsert_setting("card:v1", "{\"stage\":\"active\"}")
            .await
            .unwrap();
        assert_eq!(
            db.get_setting("card:v1").await.unwrap().as_deref(),
            Some("{\"stage\":\"active\"}")
        );
        db.upsert_setting("card:v1", "{\"stage\":\"renewal\"}")
            .await
            .unwrap();
        assert_eq!(
            db.get_setting("card:v1").await.unwrap().as_deref(),
            Some("{\"stage\":\"renewal\"}")
        );
    }

    #[tokio::test]
    async fn two_vaults_are_physically_isolated() {
        // Two separate in-memory vaults never see each other's students.
        let vault_a = VaultDb::open_in_memory().await.unwrap();
        let vault_b = VaultDb::open_in_memory().await.unwrap();
        vault_a
            .add_student("chamber-a", "Alex", Some(2027))
            .await
            .unwrap();

        assert_eq!(vault_a.list_students().await.unwrap().len(), 1);
        assert_eq!(vault_b.list_students().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn essay_version_control_increments() {
        let vault = VaultDb::open_in_memory().await.unwrap();
        let sid = vault.add_student("c1", "Jordan", None).await.unwrap();
        let v1 = vault
            .commit_essay("essay-1", &sid, "first draft", "Hello world.")
            .await
            .unwrap();
        let v2 = vault
            .commit_essay("essay-1", &sid, "tighten intro", "Hello, world!")
            .await
            .unwrap();
        assert_eq!(v1.seq, 1);
        assert_eq!(v2.seq, 2);
        let history = vault.essay_history("essay-1").await.unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].body, "Hello world.");
    }

    #[tokio::test]
    async fn milestones_sort_and_toggle() {
        let vault = VaultDb::open_in_memory().await.unwrap();
        let sid = vault.ensure_student("c1", "Student").await.unwrap();
        vault
            .add_milestone(&sid, "Common App due", Some("2026-11-01"))
            .await
            .unwrap();
        let early = vault
            .add_milestone(&sid, "Early action", Some("2026-10-15"))
            .await
            .unwrap();
        vault
            .add_milestone(&sid, "Someday task", None)
            .await
            .unwrap();

        let list = vault.list_milestones(&sid).await.unwrap();
        assert_eq!(list.len(), 3);
        // Earliest due date first; the undated one sorts last.
        assert_eq!(list[0].title, "Early action");
        assert_eq!(list[2].title, "Someday task");

        vault.set_milestone_done(&early.id, true).await.unwrap();
        let done = vault
            .list_milestones(&sid)
            .await
            .unwrap()
            .into_iter()
            .find(|m| m.id == early.id)
            .unwrap();
        assert!(done.done);
    }

    #[tokio::test]
    async fn qavault_export_import_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rivera.qavault");

        let source = VaultDb::open_in_memory().await.unwrap();
        let sid = source.add_student("c1", "Sam", Some(2026)).await.unwrap();
        source
            .commit_essay("common-app", &sid, "draft", "My story...")
            .await
            .unwrap();
        source.export_qavault(&path).await.unwrap();
        assert!(path.exists());

        let restored = VaultDb::open_in_memory().await.unwrap();
        restored.import_qavault(&path).await.unwrap();
        assert_eq!(restored.list_students().await.unwrap().len(), 1);
        assert_eq!(restored.essay_history("common-app").await.unwrap().len(), 1);
    }
}
