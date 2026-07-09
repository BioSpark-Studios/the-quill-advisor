-- vault_db: one physically isolated SQLite file per Counselor vault.
-- This is the privacy boundary — a second vault's data lives in a different
-- file this connection never opens. Classrooms and chambers nest here.

CREATE TABLE IF NOT EXISTS vault_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS students (
    id           TEXT PRIMARY KEY,
    chamber_id   TEXT NOT NULL,
    display_name TEXT NOT NULL,
    grad_year    INTEGER,
    created_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS milestones (
    id         TEXT PRIMARY KEY,
    student_id TEXT NOT NULL REFERENCES students(id) ON DELETE CASCADE,
    title      TEXT NOT NULL,
    due_at     TEXT,
    done       INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS documents (
    id         TEXT PRIMARY KEY,
    student_id TEXT NOT NULL REFERENCES students(id) ON DELETE CASCADE,
    title      TEXT NOT NULL,
    kind       TEXT NOT NULL,
    body       TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);

-- Git-inspired essay version control: each commit is an immutable row.
CREATE TABLE IF NOT EXISTS essay_versions (
    id         TEXT PRIMARY KEY,
    essay_id   TEXT NOT NULL,
    student_id TEXT NOT NULL REFERENCES students(id) ON DELETE CASCADE,
    seq        INTEGER NOT NULL,
    message    TEXT NOT NULL DEFAULT '',
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_essay_versions ON essay_versions(essay_id, seq);

-- Local billing / hour-tracking ledger (aggregated up to core only via a gate).
CREATE TABLE IF NOT EXISTS billing_entries (
    id          TEXT PRIMARY KEY,
    student_id  TEXT REFERENCES students(id) ON DELETE SET NULL,
    minutes     INTEGER NOT NULL DEFAULT 0,
    description TEXT NOT NULL DEFAULT '',
    billed_at   TEXT NOT NULL
);
