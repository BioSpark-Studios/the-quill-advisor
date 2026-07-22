//! Opens and caches the physically isolated `vault_db` for each Counselor vault.
//!
//! Every vault's data lives in its own SQLite file under `base_dir`. The manager
//! hands out (and reuses) a [`VaultDb`] handle per vault id — so a feature that
//! asks for vault A's database can never touch vault B's file. This is the
//! runtime enforcement of the physical-isolation privacy boundary.

use crate::error::Result;
use crate::vault_db::VaultDb;
use quill_core::ids::VaultId;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::Mutex;

/// A cache of open, isolated vault databases keyed by vault id.
pub struct VaultManager {
    base_dir: PathBuf,
    open: Mutex<HashMap<VaultId, VaultDb>>,
}

impl VaultManager {
    /// Create a manager rooted at `base_dir` (one `<vault_id>.db` file per vault).
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            open: Mutex::new(HashMap::new()),
        }
    }

    /// The on-disk path for a vault's database file.
    #[must_use]
    pub fn path_for(&self, id: VaultId) -> PathBuf {
        self.base_dir.join(format!("{id}.db"))
    }

    /// Get the vault's database, opening (and caching) it on first use.
    ///
    /// # Errors
    /// Fails if the directory cannot be created or the database cannot be opened.
    pub async fn vault(&self, id: VaultId) -> Result<VaultDb> {
        let mut open = self.open.lock().await;
        if let Some(db) = open.get(&id) {
            return Ok(db.clone());
        }
        std::fs::create_dir_all(&self.base_dir).ok();
        let db = VaultDb::open(self.path_for(id)).await?;
        open.insert(id, db.clone());
        Ok(db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn manager_isolates_and_caches_vaults() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = VaultManager::new(dir.path());
        let a = VaultId::new();
        let b = VaultId::new();

        // Writing to A's cached handle is visible on a second lookup of A…
        mgr.vault(a)
            .await
            .unwrap()
            .add_student("c1", "Alex", None)
            .await
            .unwrap();
        assert_eq!(
            mgr.vault(a)
                .await
                .unwrap()
                .list_students()
                .await
                .unwrap()
                .len(),
            1
        );
        // …but never leaks into B's separate file.
        assert_eq!(
            mgr.vault(b)
                .await
                .unwrap()
                .list_students()
                .await
                .unwrap()
                .len(),
            0
        );

        // Each vault got its own file on disk.
        assert!(mgr.path_for(a).exists());
        assert!(mgr.path_for(b).exists());
        assert_ne!(mgr.path_for(a), mgr.path_for(b));
    }
}
