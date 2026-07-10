-- core_db: the global Level-0 Master store.
-- Holds everything shared across the whole installation. Per-vault student data
-- lives in physically separate vault_db files, never here.

CREATE TABLE IF NOT EXISTS settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL
);

-- Registry of every node in the vault hierarchy (Master..Chamber). `db_path` is
-- the isolated vault_db file for Counselor-level roots; NULL for Master and for
-- nested nodes that live inside their counselor's vault_db.
CREATE TABLE IF NOT EXISTS vault_directory (
    id          TEXT PRIMARY KEY,
    parent_id   TEXT REFERENCES vault_directory(id) ON DELETE CASCADE,
    level       INTEGER NOT NULL CHECK (level BETWEEN 0 AND 3),
    name        TEXT NOT NULL,
    db_path     TEXT,
    gate_config TEXT NOT NULL DEFAULT '{"contracts":[]}',
    created_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_vault_parent ON vault_directory(parent_id);

-- License registry drives Bronze/Silver/Gold tier limits.
CREATE TABLE IF NOT EXISTS license (
    key          TEXT PRIMARY KEY,
    tier         TEXT NOT NULL DEFAULT 'bronze',
    max_vaults   INTEGER NOT NULL DEFAULT 1,
    max_seats    INTEGER NOT NULL DEFAULT 3,
    max_students INTEGER NOT NULL DEFAULT 25,
    issued_at    TEXT NOT NULL
);

-- Human actors and their global role.
CREATE TABLE IF NOT EXISTS actors (
    id           TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    role         TEXT NOT NULL,
    created_at   TEXT NOT NULL
);

-- Per-vault RBAC grants (an actor's role within a specific vault subtree).
CREATE TABLE IF NOT EXISTS rbac_grants (
    actor_id  TEXT NOT NULL REFERENCES actors(id) ON DELETE CASCADE,
    vault_id  TEXT NOT NULL REFERENCES vault_directory(id) ON DELETE CASCADE,
    role      TEXT NOT NULL,
    PRIMARY KEY (actor_id, vault_id)
);

-- Scoped, revocable parent-portal access tokens.
CREATE TABLE IF NOT EXISTS parent_tokens (
    token      TEXT PRIMARY KEY,
    chamber_id TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

-- Counselor-led AI authorization (per-chamber gate toggles).
CREATE TABLE IF NOT EXISTS ai_authorization (
    chamber_id        TEXT PRIMARY KEY,
    enabled           INTEGER NOT NULL DEFAULT 0,
    authorized_agents TEXT NOT NULL DEFAULT '[]'
);

-- Append-only audit trail. Every gated access and privileged action lands here.
CREATE TABLE IF NOT EXISTS audit_log (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    at       TEXT NOT NULL,
    actor_id TEXT,
    vault_id TEXT NOT NULL,
    action   TEXT NOT NULL,
    detail   TEXT NOT NULL,
    allowed  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_audit_vault ON audit_log(vault_id);
