import { useEffect, useState } from "react";
import { api, type FieldSpec, type PanelKind, type PluginRecord } from "../lib/ipc";
import type { PluginProps } from "./registry";

/**
 * Generic renderer for a declarative (Forge) plugin. It turns the manifest's UI
 * schema into a working panel — no plugin-specific code — persisting records
 * through the capability-gated plugin record store.
 */
export function DeclarativePlugin({ manifest, vault, chamberId, onClose }: PluginProps) {
  const ui = manifest.kind.type === "declarative" ? manifest.kind.ui : { panels: [] };
  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex h-[620px] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border border-border bg-surface-raised shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <div className="flex items-center gap-2">
            <span className="text-lg">{manifest.icon}</span>
            <h2 className="font-serif text-lg text-ink">{manifest.name}</h2>
            <span className="rounded-full border border-border px-2 py-0.5 text-[10px] uppercase tracking-wide text-ink-muted">
              plugin
            </span>
          </div>
          <button onClick={onClose} className="text-ink-muted hover:text-ink" aria-label="Close">
            ✕
          </button>
        </div>
        <div className="flex-1 space-y-6 overflow-y-auto p-5">
          {ui.panels.map((panel, i) => (
            <section key={i}>
              <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-ink-muted">
                {panel.title}
              </h3>
              <PanelBody
                panel={panel.kind}
                pluginId={manifest.id}
                vaultId={vault.id}
                chamberId={chamberId}
              />
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}

function PanelBody({
  panel,
  pluginId,
  vaultId,
  chamberId,
}: {
  panel: PanelKind;
  pluginId: string;
  vaultId: string;
  chamberId: string;
}) {
  if (panel.type === "note") {
    return <p className="whitespace-pre-wrap text-sm text-ink">{panel.content}</p>;
  }
  return (
    <CollectionPanel panel={panel} pluginId={pluginId} vaultId={vaultId} chamberId={chamberId} />
  );
}

function CollectionPanel({
  panel,
  pluginId,
  vaultId,
  chamberId,
}: {
  panel: Extract<PanelKind, { type: "collection" }>;
  pluginId: string;
  vaultId: string;
  chamberId: string;
}) {
  const [records, setRecords] = useState<PluginRecord[]>([]);
  const [form, setForm] = useState<Record<string, unknown>>({});
  const [busy, setBusy] = useState(false);

  async function refresh() {
    setRecords(await api.pluginRecordList(vaultId, chamberId, pluginId, panel.collection));
  }

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pluginId, panel.collection, chamberId]);

  const missingRequired = panel.fields.some(
    (f) => f.required && !String(form[f.key] ?? "").trim(),
  );

  async function add() {
    if (missingRequired || busy) return;
    setBusy(true);
    try {
      await api.pluginRecordAdd(vaultId, chamberId, pluginId, panel.collection, form);
      setForm({});
      await refresh();
    } finally {
      setBusy(false);
    }
  }

  async function remove(id: string) {
    setRecords((prev) => prev.filter((r) => r.id !== id));
    await api.pluginRecordDelete(vaultId, id);
  }

  return (
    <div className="space-y-4">
      <div className="grid gap-2 rounded-xl border border-border bg-surface p-3">
        {panel.fields.map((f) => (
          <Field
            key={f.key}
            spec={f}
            value={form[f.key]}
            onChange={(v) => setForm((prev) => ({ ...prev, [f.key]: v }))}
          />
        ))}
        <button
          onClick={add}
          disabled={busy || missingRequired}
          className="mt-1 self-start rounded-lg bg-primary px-4 py-1.5 text-sm font-medium text-surface-raised disabled:opacity-50"
        >
          {panel.add_label}
        </button>
      </div>

      <ul className="space-y-2">
        {records.map((r) => (
          <li
            key={r.id}
            className="flex items-start justify-between gap-3 rounded-xl border border-border bg-surface-raised/70 p-3"
          >
            <div>
              <div className="font-serif text-sm text-ink">
                {String(r.data[panel.title_field] ?? "(untitled)")}
              </div>
              {panel.subtitle_field && r.data[panel.subtitle_field] != null && (
                <div className="text-xs text-ink-muted">{String(r.data[panel.subtitle_field])}</div>
              )}
            </div>
            <button
              onClick={() => remove(r.id)}
              className="text-ink-muted hover:text-red-400"
              aria-label="Delete"
            >
              ✕
            </button>
          </li>
        ))}
        {records.length === 0 && <li className="text-sm text-ink-muted">No entries yet.</li>}
      </ul>
    </div>
  );
}

function Field({
  spec,
  value,
  onChange,
}: {
  spec: FieldSpec;
  value: unknown;
  onChange: (v: unknown) => void;
}) {
  const label = (
    <span className="text-xs text-ink-muted">
      {spec.label}
      {spec.required ? " *" : ""}
    </span>
  );
  const cls =
    "rounded-lg border border-border bg-surface-raised px-3 py-1.5 text-sm text-ink outline-none focus:border-primary";

  if (spec.kind.type === "long_text") {
    return (
      <label className="grid gap-1">
        {label}
        <textarea
          className={`${cls} min-h-[70px] resize-y`}
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value)}
        />
      </label>
    );
  }
  if (spec.kind.type === "bool") {
    return (
      <label className="flex items-center gap-2">
        <input type="checkbox" checked={Boolean(value)} onChange={(e) => onChange(e.target.checked)} />
        {label}
      </label>
    );
  }
  if (spec.kind.type === "select") {
    return (
      <label className="grid gap-1">
        {label}
        <select className={cls} value={String(value ?? "")} onChange={(e) => onChange(e.target.value)}>
          <option value="">—</option>
          {spec.kind.options.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>
      </label>
    );
  }
  return (
    <label className="grid gap-1">
      {label}
      <input
        type={spec.kind.type === "date" ? "date" : "text"}
        className={cls}
        value={String(value ?? "")}
        onChange={(e) => onChange(e.target.value)}
      />
    </label>
  );
}
