import { useState } from "react";
import {
  api,
  type AvailablePlugin,
  type FieldSpec,
  type VaultComposition,
} from "../../lib/ipc";

const CAP_LABELS: Record<string, string> = {
  read_student_data: "Read student data",
  write_student_data: "Modify student data",
  read_essays: "Read essays",
  write_essays: "Write essays",
  read_milestones: "Read milestones",
  write_milestones: "Manage milestones",
  use_ai: "Use AI",
  read_billing: "Read billing",
  write_billing: "Write billing",
  store_plugin_records: "Store its own records",
};

function priceLabel(p: AvailablePlugin["manifest"]["pricing"]): string {
  return p.type === "free" ? "Free" : `$${(p.price_cents / 100).toFixed(2)} · ${p.tier}`;
}

/**
 * The BioSpark Forge manager: browse every plugin (built-in + store), see its
 * capabilities, trust badge, and price, install store plugins, and enable or
 * disable them in this vault. This is the vault-composition control surface —
 * add and remove features at will.
 */
export function AddonManager({
  vault,
  available,
  composition,
  onChange,
  onCatalogChange,
  onClose,
}: {
  vault: { id: string; name: string };
  available: AvailablePlugin[];
  composition: VaultComposition;
  onChange: (next: VaultComposition) => void;
  onCatalogChange: () => Promise<void>;
  onClose: () => void;
}) {
  const [busy, setBusy] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState<string | null>(null);

  const enabledIds = new Set(composition.plugins.map((p) => p.plugin_id));

  function settingsOf(pluginId: string): Record<string, unknown> {
    const p = composition.plugins.find((x) => x.plugin_id === pluginId);
    return (p?.settings && typeof p.settings === "object" ? p.settings : {}) as Record<
      string,
      unknown
    >;
  }

  function setSettings(pluginId: string, next: Record<string, unknown>) {
    onChange({
      ...composition,
      plugins: composition.plugins.map((p) =>
        p.plugin_id === pluginId ? { ...p, settings: next } : p,
      ),
    });
  }

  function toggleEnabled(p: AvailablePlugin) {
    if (enabledIds.has(p.manifest.id)) {
      onChange({ ...composition, plugins: composition.plugins.filter((x) => x.plugin_id !== p.manifest.id) });
    } else {
      onChange({
        ...composition,
        plugins: [
          ...composition.plugins,
          {
            plugin_id: p.manifest.id,
            settings: null,
            layout: p.manifest.default_layout,
            order: composition.plugins.length,
          },
        ],
      });
    }
  }

  async function install(id: string) {
    setBusy(id);
    try {
      await api.installPlugin(id);
      await onCatalogChange();
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex h-[680px] w-full max-w-3xl flex-col overflow-hidden rounded-2xl border border-border bg-surface-raised shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <div>
            <h2 className="font-serif text-lg text-ink">BioSpark Forge · Plugins</h2>
            <p className="text-xs text-ink-muted">{vault.name} — add or remove features at will</p>
          </div>
          <button onClick={onClose} className="text-ink-muted hover:text-ink" aria-label="Close">
            ✕
          </button>
        </div>

        <div className="flex-1 space-y-3 overflow-y-auto p-4">
          {available.map((p) => {
            const enabled = enabledIds.has(p.manifest.id);
            const needsInstall = p.source === "forge" && !p.installed;
            const configurable = enabled && p.manifest.config_schema.length > 0;
            const showingSettings = settingsOpen === p.manifest.id;
            return (
              <div key={p.manifest.id} className="rounded-xl border border-border bg-surface">
              <div
                className="flex items-start gap-3 p-4"
              >
                <div className="text-2xl">{p.manifest.icon}</div>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <h3 className="font-serif text-base text-ink">{p.manifest.name}</h3>
                    <TrustBadge trust={p.trust} />
                    <span className="text-[11px] text-ink-muted">by {p.manifest.author}</span>
                    <span className="rounded-full bg-primary/10 px-2 py-0.5 text-[11px] text-primary">
                      {priceLabel(p.manifest.pricing)}
                    </span>
                    {p.source === "builtin" && (
                      <span className="rounded-full border border-border px-2 py-0.5 text-[10px] uppercase tracking-wide text-ink-muted">
                        core
                      </span>
                    )}
                  </div>
                  <p className="mt-1 text-sm text-ink-muted">{p.manifest.description}</p>
                  {p.manifest.capabilities.length > 0 && (
                    <div className="mt-2 flex flex-wrap gap-1.5">
                      {p.manifest.capabilities.map((c) => (
                        <span
                          key={c}
                          className="rounded-full border border-border px-2 py-0.5 text-[11px] text-ink-muted"
                        >
                          {CAP_LABELS[c] ?? c}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  {configurable && (
                    <button
                      onClick={() => setSettingsOpen(showingSettings ? null : p.manifest.id)}
                      className={`rounded-lg border px-2 py-1 text-sm ${
                        showingSettings
                          ? "border-primary text-primary"
                          : "border-border text-ink-muted hover:text-ink"
                      }`}
                      title="Plugin settings"
                    >
                      ⚙
                    </button>
                  )}
                  {needsInstall ? (
                    <button
                      onClick={() => install(p.manifest.id)}
                      disabled={busy === p.manifest.id}
                      className="rounded-lg bg-primary px-3 py-1.5 text-sm font-medium text-surface-raised disabled:opacity-50"
                    >
                      {busy === p.manifest.id ? "Installing…" : "Install"}
                    </button>
                  ) : (
                    <button
                      onClick={() => toggleEnabled(p)}
                      role="switch"
                      aria-checked={enabled}
                      className={`relative h-6 w-11 rounded-full transition-colors ${
                        enabled ? "bg-primary" : "bg-border"
                      }`}
                      title={enabled ? "Enabled in this vault" : "Disabled"}
                    >
                      <span
                        className={`absolute top-0.5 h-5 w-5 rounded-full bg-white transition-transform ${
                          enabled ? "translate-x-5" : "translate-x-0.5"
                        }`}
                      />
                    </button>
                  )}
                </div>
              </div>

              {configurable && showingSettings && (
                <div className="border-t border-border p-4">
                  <h4 className="mb-2 text-xs font-semibold uppercase tracking-wide text-ink-muted">
                    Settings — {vault.name}
                  </h4>
                  <SettingsForm
                    schema={p.manifest.config_schema}
                    values={settingsOf(p.manifest.id)}
                    onChange={(next) => setSettings(p.manifest.id, next)}
                  />
                </div>
              )}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

/** Schema-driven per-vault settings editor for an enabled plugin. */
function SettingsForm({
  schema,
  values,
  onChange,
}: {
  schema: FieldSpec[];
  values: Record<string, unknown>;
  onChange: (next: Record<string, unknown>) => void;
}) {
  const cls =
    "rounded-lg border border-border bg-surface-raised px-3 py-1.5 text-sm text-ink outline-none focus:border-primary";
  function set(key: string, v: unknown) {
    onChange({ ...values, [key]: v });
  }
  return (
    <div className="grid gap-3">
      {schema.map((f) => (
        <label key={f.key} className="grid gap-1">
          <span className="text-xs text-ink-muted">{f.label}</span>
          {f.kind.type === "select" ? (
            <select className={cls} value={String(values[f.key] ?? "")} onChange={(e) => set(f.key, e.target.value)}>
              <option value="">—</option>
              {f.kind.options.map((o) => (
                <option key={o} value={o}>
                  {o}
                </option>
              ))}
            </select>
          ) : f.kind.type === "bool" ? (
            <input
              type="checkbox"
              className="h-4 w-4"
              checked={Boolean(values[f.key])}
              onChange={(e) => set(f.key, e.target.checked)}
            />
          ) : (
            <input
              type={f.kind.type === "date" ? "date" : "text"}
              className={cls}
              value={String(values[f.key] ?? "")}
              onChange={(e) => set(f.key, e.target.value)}
            />
          )}
        </label>
      ))}
    </div>
  );
}

function TrustBadge({ trust }: { trust: AvailablePlugin["trust"] }) {
  const map = {
    verified: { label: "✓ Verified", cls: "bg-emerald-500/15 text-emerald-500" },
    unsigned: { label: "Unsigned", cls: "bg-border/40 text-ink-muted" },
    untrusted: { label: "⚠ Untrusted", cls: "bg-red-500/15 text-red-400" },
  }[trust];
  return (
    <span className={`rounded-full px-2 py-0.5 text-[11px] font-medium ${map.cls}`}>{map.label}</span>
  );
}
