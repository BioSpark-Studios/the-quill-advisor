//! An isolated, per-Counselor `vault_db`.
//!
//! Each Counselor vault gets its own SQLite file. Because a `VaultDb` only ever
//! opens one file, data in a *different* vault is physically unreachable through
//! this handle — that file isolation is the privacy boundary the whole product
//! is built around. `.qavault` export/import serializes a vault to a portable
//! JSON snapshot.

use crate::error::{Result, StorageError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;

static VAULT_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/vault");

/// A student record inside a vault.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Student {
    /// Student id (UUID string).
    pub id: String,
    /// The chamber this student inhabits.
    pub chamber_id: String,
    /// Display name.
    pub display_name: String,
    /// Graduation year, if known.
    pub grad_year: Option<i64>,
}

/// One immutable essay revision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EssayVersion {
    /// Version id.
    pub id: String,
    /// The logical essay this revision belongs to.
    pub essay_id: String,
    /// Owning student.
    pub student_id: String,
    /// Monotonic sequence number (1-based).
    pub seq: i64,
    /// Commit message.
    pub message: String,
    /// Full essay body at this revision.
    pub body: String,
}

/// Handle to a single isolated vault database.
#[derive(Clone)]
pub struct VaultDb {
    pool: SqlitePool,
}

impl VaultDb {
    /// Open (creating if absent) a vault_db at `path` in WAL mode and migrate it.
    ///
    /// # Errors
    /// Fails if the file cannot be opened or a migration errors.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let opts = SqliteConnectOptions::new()
            .filename(path.as_ref())
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await?;
        VAULT_MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    /// Open an isolated in-memory vault_db (tests).
    ///
    /// # Errors
    /// Fails if the database cannot be created or migrated.
    pub async fn open_in_memory() -> Result<Self> {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        VAULT_MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    /// Add a student, returning its generated id.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn add_student(
        &self,
        chamber_id: &str,
        display_name: &str,
        grad_year: Option<i64>,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO students (id, chamber_id, display_name, grad_year, created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(chamber_id)
        .bind(display_name)
        .bind(grad_year)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    /// List all students in this vault.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn list_students(&self) -> Result<Vec<Student>> {
        let rows = sqlx::query(
            "SELECT id, chamber_id, display_name, grad_year FROM students ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| Student {
                id: r.get("id"),
                chamber_id: r.get("chamber_id"),
                display_name: r.get("display_name"),
                grad_year: r.get("grad_year"),
            })
            .collect())
    }

    /// Commit a new essay revision, auto-incrementing the sequence number.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn commit_essay(
        &self,
        essay_id: &str,
        student_id: &str,
        message: &str,
        body: &str,
    ) -> Result<EssayVersion> {
        let next: i64 = sqlx::query(
            "SELECT COALESCE(MAX(seq), 0) + 1 AS n FROM essay_versions WHERE essay_id = ?",
        )
        .bind(essay_id)
        .fetch_one(&self.pool)
        .await?
        .get("n");
        let version = EssayVersion {
            id: uuid::Uuid::new_v4().to_string(),
            essay_id: essay_id.to_string(),
            student_id: student_id.to_string(),
            seq: next,
            message: message.to_string(),
            body: body.to_string(),
        };
        sqlx::query(
            "INSERT INTO essay_versions (id, essay_id, student_id, seq, message, body, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&version.id)
        .bind(&version.essay_id)
        .bind(&version.student_id)
        .bind(version.seq)
        .bind(&version.message)
        .bind(&version.body)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(version)
    }

    /// Full revision history for an essay, oldest first.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn essay_history(&self, essay_id: &str) -> Result<Vec<EssayVersion>> {
        let rows = sqlx::query(
            "SELECT id, essay_id, student_id, seq, message, body
             FROM essay_versions WHERE essay_id = ? ORDER BY seq",
        )
        .bind(essay_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| EssayVersion {
                id: r.get("id"),
                essay_id: r.get("essay_id"),
                student_id: r.get("student_id"),
                seq: r.get("seq"),
                message: r.get("message"),
                body: r.get("body"),
            })
            .collect())
    }

    /// Export this vault to a portable `.qavault` JSON snapshot on disk.
    ///
    /// # Errors
    /// Propagates database, serialization, and file I/O errors.
    pub async fn export_qavault(&self, out: impl AsRef<Path>) -> Result<QaVault> {
        let snapshot = QaVault {
            format: "qavault/1".to_string(),
            students: self.list_students().await?,
            essays: self.all_essay_versions().await?,
        };
        let json = serde_json::to_string_pretty(&snapshot)?;
        std::fs::write(out.as_ref(), json)
            .map_err(|e| StorageError::InvalidVaultPath(e.to_string()))?;
        Ok(snapshot)
    }

    /// Import a `.qavault` snapshot into this (typically fresh) vault.
    ///
    /// # Errors
    /// Propagates file I/O, deserialization, and database errors.
    pub async fn import_qavault(&self, from: impl AsRef<Path>) -> Result<()> {
        let json = std::fs::read_to_string(from.as_ref())
            .map_err(|e| StorageError::InvalidVaultPath(e.to_string()))?;
        let snapshot: QaVault = serde_json::from_str(&json)?;
        for s in &snapshot.students {
            sqlx::query(
                "INSERT OR REPLACE INTO students (id, chamber_id, display_name, grad_year, created_at)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&s.id)
            .bind(&s.chamber_id)
            .bind(&s.display_name)
            .bind(s.grad_year)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        }
        for v in &snapshot.essays {
            sqlx::query(
                "INSERT OR REPLACE INTO essay_versions (id, essay_id, student_id, seq, message, body, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&v.id)
            .bind(&v.essay_id)
            .bind(&v.student_id)
            .bind(v.seq)
            .bind(&v.message)
            .bind(&v.body)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn all_essay_versions(&self) -> Result<Vec<EssayVersion>> {
        let rows = sqlx::query(
            "SELECT id, essay_id, student_id, seq, message, body FROM essay_versions ORDER BY essay_id, seq",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| EssayVersion {
                id: r.get("id"),
                essay_id: r.get("essay_id"),
                student_id: r.get("student_id"),
                seq: r.get("seq"),
                message: r.get("message"),
                body: r.get("body"),
            })
            .collect())
    }
}

/// The portable `.qavault` snapshot format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QaVault {
    /// Format tag for forward compatibility.
    pub format: String,
    /// All students in the vault.
    pub students: Vec<Student>,
    /// All essay revisions in the vault.
    pub essays: Vec<EssayVersion>,
}
