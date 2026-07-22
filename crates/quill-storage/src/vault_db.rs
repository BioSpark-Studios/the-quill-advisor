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

/// A generic record owned by a declarative (Forge) plugin. `data` is an opaque
/// JSON string whose shape is defined by the plugin's UI schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginRecord {
    /// Record id.
    pub id: String,
    /// The plugin's stored JSON payload.
    pub data: String,
}

/// A named essay prompt slot (e.g. "Common App", "Why Cornell?"). The essay's
/// revision history is keyed by this id in `essay_versions`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EssaySlot {
    /// Stable id used as the `essay_id` in `essay_versions`.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Word limit shown against the draft's live word count, if the prompt has one.
    pub word_limit: Option<i64>,
}

/// One entry in the vault's hour-tracking ledger: time worked against the
/// family's retainer, optionally attributed to a specific student.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BillingEntry {
    /// Entry id.
    pub id: String,
    /// The student this time was spent on, if attributable to one.
    pub student_id: Option<String>,
    /// Minutes of work this entry logs. Stored in minutes for precision;
    /// the UI converts to hours for display.
    pub minutes: i64,
    /// What the time was for (e.g. "Essay review call").
    pub description: String,
    /// ISO `YYYY-MM-DD` date the work was done.
    pub billed_at: String,
}

/// An application milestone / deadline on a student's timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Milestone {
    /// Milestone id.
    pub id: String,
    /// Owning student.
    pub student_id: String,
    /// What is due.
    pub title: String,
    /// Optional due date (ISO `YYYY-MM-DD`).
    pub due_at: Option<String>,
    /// Whether it's been completed.
    pub done: bool,
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

    /// Get the id of the (first) student in a chamber, creating one if none
    /// exists yet. Essays are owned by a student, so this backs the common case
    /// of "the student in this chamber."
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn ensure_student(&self, chamber_id: &str, display_name: &str) -> Result<String> {
        let existing: Option<String> =
            sqlx::query("SELECT id FROM students WHERE chamber_id = ? ORDER BY created_at LIMIT 1")
                .bind(chamber_id)
                .fetch_optional(&self.pool)
                .await?
                .map(|r| r.get("id"));
        if let Some(id) = existing {
            return Ok(id);
        }
        self.add_student(chamber_id, display_name, None).await
    }

    /// Add a milestone for a student, returning it.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn add_milestone(
        &self,
        student_id: &str,
        title: &str,
        due_at: Option<&str>,
    ) -> Result<Milestone> {
        let milestone = Milestone {
            id: uuid::Uuid::new_v4().to_string(),
            student_id: student_id.to_string(),
            title: title.to_string(),
            due_at: due_at.map(ToString::to_string),
            done: false,
        };
        sqlx::query(
            "INSERT INTO milestones (id, student_id, title, due_at, done, created_at)
             VALUES (?, ?, ?, ?, 0, ?)",
        )
        .bind(&milestone.id)
        .bind(&milestone.student_id)
        .bind(&milestone.title)
        .bind(&milestone.due_at)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(milestone)
    }

    /// List a student's milestones, undated last, then by due date.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn list_milestones(&self, student_id: &str) -> Result<Vec<Milestone>> {
        let rows = sqlx::query(
            "SELECT id, student_id, title, due_at, done FROM milestones
             WHERE student_id = ? ORDER BY due_at IS NULL, due_at, created_at",
        )
        .bind(student_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| Milestone {
                id: r.get("id"),
                student_id: r.get("student_id"),
                title: r.get("title"),
                due_at: r.get("due_at"),
                done: r.get::<i64, _>("done") != 0,
            })
            .collect())
    }

    /// Mark a milestone done or not-done.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn set_milestone_done(&self, id: &str, done: bool) -> Result<()> {
        sqlx::query("UPDATE milestones SET done = ? WHERE id = ?")
            .bind(i64::from(done))
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Read a value from `vault_meta`, the vault's local key/value store.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT value FROM vault_meta WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get::<String, _>("value")))
    }

    /// Insert or update a value in `vault_meta`.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO vault_meta (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// List the vault's custom essay slots (on top of the three defaults the UI
    /// always offers). Empty until the counselor adds one.
    ///
    /// # Errors
    /// Propagates database or deserialization errors.
    pub async fn list_essay_slots(&self) -> Result<Vec<EssaySlot>> {
        match self.get_meta("essay_slots").await? {
            Some(json) => Ok(serde_json::from_str(&json)?),
            None => Ok(Vec::new()),
        }
    }

    /// Add a custom essay slot (e.g. a school-specific supplement), returning it.
    ///
    /// # Errors
    /// Propagates database or serialization errors.
    pub async fn add_essay_slot(&self, label: &str, word_limit: Option<i64>) -> Result<EssaySlot> {
        let mut slots = self.list_essay_slots().await?;
        let slot = EssaySlot {
            id: format!("custom-{}", uuid::Uuid::new_v4()),
            label: label.to_string(),
            word_limit,
        };
        slots.push(slot.clone());
        self.set_meta("essay_slots", &serde_json::to_string(&slots)?)
            .await?;
        Ok(slot)
    }

    /// Remove a custom essay slot. Its committed revisions are left in place —
    /// only the slot's visibility in the picker is removed.
    ///
    /// # Errors
    /// Propagates database or serialization errors.
    pub async fn delete_essay_slot(&self, id: &str) -> Result<()> {
        let slots: Vec<EssaySlot> = self
            .list_essay_slots()
            .await?
            .into_iter()
            .filter(|s| s.id != id)
            .collect();
        self.set_meta("essay_slots", &serde_json::to_string(&slots)?)
            .await?;
        Ok(())
    }

    /// The family's total purchased retainer, in minutes. Defaults to 0 (no
    /// retainer set yet) so a fresh vault shows an empty ledger rather than
    /// erroring.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn retainer_minutes(&self) -> Result<i64> {
        Ok(self
            .get_meta("billing_retainer_minutes")
            .await?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0))
    }

    /// Set the family's total purchased retainer, in minutes (e.g. topping up
    /// after they buy another block of hours).
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn set_retainer_minutes(&self, minutes: i64) -> Result<()> {
        self.set_meta("billing_retainer_minutes", &minutes.to_string())
            .await
    }

    /// Log time worked against the retainer, returning the new entry.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn log_billing_entry(
        &self,
        student_id: Option<&str>,
        minutes: i64,
        description: &str,
        billed_at: &str,
    ) -> Result<BillingEntry> {
        let entry = BillingEntry {
            id: uuid::Uuid::new_v4().to_string(),
            student_id: student_id.map(ToString::to_string),
            minutes,
            description: description.to_string(),
            billed_at: billed_at.to_string(),
        };
        sqlx::query(
            "INSERT INTO billing_entries (id, student_id, minutes, description, billed_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&entry.id)
        .bind(&entry.student_id)
        .bind(entry.minutes)
        .bind(&entry.description)
        .bind(&entry.billed_at)
        .execute(&self.pool)
        .await?;
        Ok(entry)
    }

    /// List every ledger entry, most recent first.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn list_billing_entries(&self) -> Result<Vec<BillingEntry>> {
        let rows = sqlx::query(
            "SELECT id, student_id, minutes, description, billed_at
             FROM billing_entries ORDER BY billed_at DESC, rowid DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| BillingEntry {
                id: r.get("id"),
                student_id: r.get("student_id"),
                minutes: r.get("minutes"),
                description: r.get("description"),
                billed_at: r.get("billed_at"),
            })
            .collect())
    }

    /// Delete a ledger entry.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn delete_billing_entry(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM billing_entries WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Total minutes logged against the retainer so far.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn billing_minutes_used(&self) -> Result<i64> {
        let used: Option<i64> = sqlx::query("SELECT SUM(minutes) AS used FROM billing_entries")
            .fetch_one(&self.pool)
            .await?
            .get("used");
        Ok(used.unwrap_or(0))
    }

    /// Add a plugin record into a namespaced collection.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn plugin_record_add(
        &self,
        plugin_id: &str,
        collection: &str,
        chamber_id: Option<&str>,
        data: &str,
    ) -> Result<PluginRecord> {
        let record = PluginRecord {
            id: uuid::Uuid::new_v4().to_string(),
            data: data.to_string(),
        };
        sqlx::query(
            "INSERT INTO plugin_records (id, plugin_id, collection, chamber_id, data, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.id)
        .bind(plugin_id)
        .bind(collection)
        .bind(chamber_id)
        .bind(&record.data)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(record)
    }

    /// List a plugin collection's records (optionally scoped to a chamber),
    /// oldest first.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn plugin_record_list(
        &self,
        plugin_id: &str,
        collection: &str,
        chamber_id: Option<&str>,
    ) -> Result<Vec<PluginRecord>> {
        let rows = sqlx::query(
            "SELECT id, data FROM plugin_records
             WHERE plugin_id = ? AND collection = ?
               AND (?3 IS NULL OR chamber_id = ?3)
             ORDER BY created_at",
        )
        .bind(plugin_id)
        .bind(collection)
        .bind(chamber_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| PluginRecord {
                id: r.get("id"),
                data: r.get("data"),
            })
            .collect())
    }

    /// Delete a plugin record by id.
    ///
    /// # Errors
    /// Propagates database errors.
    pub async fn plugin_record_delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM plugin_records WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
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
            retainer_minutes: self.retainer_minutes().await?,
            billing_entries: self.list_billing_entries().await?,
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
        self.set_retainer_minutes(snapshot.retainer_minutes).await?;
        for e in &snapshot.billing_entries {
            sqlx::query(
                "INSERT OR REPLACE INTO billing_entries (id, student_id, minutes, description, billed_at)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&e.id)
            .bind(&e.student_id)
            .bind(e.minutes)
            .bind(&e.description)
            .bind(&e.billed_at)
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
    /// The family's total purchased retainer, in minutes.
    #[serde(default)]
    pub retainer_minutes: i64,
    /// All billing ledger entries in the vault.
    #[serde(default)]
    pub billing_entries: Vec<BillingEntry>,
}
