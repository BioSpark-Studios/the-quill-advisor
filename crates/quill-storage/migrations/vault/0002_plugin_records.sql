-- Generic, namespaced record store for declarative (Forge) plugins.
-- Each plugin gets its own `plugin_id`/`collection` namespace inside the vault's
-- isolated database, so a data-defined plugin can persist records without any
-- bespoke Rust. Access is gated at the command layer by the plugin's declared
-- capabilities.

CREATE TABLE IF NOT EXISTS plugin_records (
    id         TEXT PRIMARY KEY,
    plugin_id  TEXT NOT NULL,
    collection TEXT NOT NULL,
    chamber_id TEXT,
    data       TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_plugin_records
    ON plugin_records(plugin_id, collection, chamber_id);
