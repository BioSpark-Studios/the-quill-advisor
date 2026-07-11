import { useEffect, useState } from "react";
import { api, type PluginRecord } from "../../lib/ipc";
import type { PluginProps } from "../../plugins/registry";

const COLLECTION = "plan_items";
const GRADES = ["9", "10", "11", "12"] as const;
const CATEGORIES = ["Course", "Activity", "Milestone", "Test"] as const;

const CATEGORY_STYLE: Record<string, string> = {
  Course: "bg-sky-500/15 text-sky-400",
  Activity: "bg-emerald-500/15 text-emerald-400",
  Milestone: "bg-amber-500/15 text-amber-500",
  Test: "bg-violet-500/15 text-violet-400",
};

/**
 * Pathway Blueprint — a four-year academic plan. Courses, activities, tests, and
 * milestones are laid out grade by grade, backed by the vault's capability-gated
 * plugin record store.
 */
export function PathwayBlueprint({ manifest, vault, chamberId, onClose }: PluginProps) {
  const [items, setItems] = useState<PluginRecord[]>([]);
  const [grade, setGrade] = useState<string>("9");
  const [category, setCategory] = useState<string>("Course");
  const [title, setTitle] = useState("");
  const [busy, setBusy] = useState(false);

  async function refresh() {
    setItems(await api.pluginRecordList(vault.id, chamberId, manifest.id, COLLECTION));
  }

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [chamberId]);

  async function add() {
    if (!title.trim() || busy) return;
    setBusy(true);
    try {
      await api.pluginRecordAdd(vault.id, chamberId, manifest.id, COLLECTION, {
        grade,
        category,
        title: title.trim(),
      });
      setTitle("");
      await refresh();
    } finally {
      setBusy(false);
    }
  }

  async function remove(id: string) {
    setItems((prev) => prev.filter((r) => r.id !== id));
    await api.pluginRecordDelete(vault.id, id);
  }

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex h-[680px] w-full max-w-5xl flex-col overflow-hidden rounded-2xl border border-border bg-surface-raised shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <div>
            <h2 className="font-serif text-lg text-ink">Pathway Blueprint</h2>
            <p className="text-xs text-ink-muted">{vault.name} · four-year plan</p>
          </div>
          <button onClick={onClose} className="text-ink-muted hover:text-ink" aria-label="Close">
            ✕
          </button>
        </div>

        {/* Add form */}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            add();
          }}
          className="flex flex-wrap items-center gap-2 border-b border-border p-3"
        >
          <select
            value={grade}
            onChange={(e) => setGrade(e.target.value)}
            className="rounded-lg border border-border bg-surface px-2 py-1.5 text-sm text-ink outline-none"
          >
            {GRADES.map((g) => (
              <option key={g} value={g}>
                Grade {g}
              </option>
            ))}
          </select>
          <select
            value={category}
            onChange={(e) => setCategory(e.target.value)}
            className="rounded-lg border border-border bg-surface px-2 py-1.5 text-sm text-ink outline-none"
          >
            {CATEGORIES.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
          <input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="e.g. 'AP Biology' or 'Debate team captain'"
            className="min-w-[220px] flex-1 rounded-lg border border-border bg-surface px-3 py-1.5 text-sm text-ink outline-none focus:border-primary"
          />
          <button
            type="submit"
            disabled={busy}
            className="rounded-lg bg-primary px-4 py-1.5 text-sm font-medium text-surface-raised disabled:opacity-50"
          >
            Add to plan
          </button>
        </form>

        {/* Grade columns */}
        <div className="grid flex-1 grid-cols-1 gap-3 overflow-y-auto p-4 md:grid-cols-2 xl:grid-cols-4">
          {GRADES.map((g) => {
            const col = items.filter((r) => String(r.data.grade) === g);
            return (
              <section
                key={g}
                className="flex flex-col rounded-xl border border-border bg-surface/60 p-3"
              >
                <div className="mb-2 flex items-center justify-between px-1">
                  <h3 className="text-sm font-semibold uppercase tracking-wide text-ink-muted">
                    Grade {g}
                  </h3>
                  <span className="rounded-full bg-primary/10 px-2 py-0.5 text-xs text-primary">
                    {col.length}
                  </span>
                </div>
                <ul className="flex flex-1 flex-col gap-2">
                  {col.map((r) => (
                    <li
                      key={r.id}
                      className="group flex items-start justify-between gap-2 rounded-lg border border-border bg-surface-raised/70 p-2.5"
                    >
                      <div className="min-w-0">
                        <span
                          className={`inline-block rounded-full px-2 py-0.5 text-[10px] font-medium ${
                            CATEGORY_STYLE[String(r.data.category)] ?? "bg-border/40 text-ink-muted"
                          }`}
                        >
                          {String(r.data.category)}
                        </span>
                        <div className="mt-1 text-sm text-ink">{String(r.data.title)}</div>
                      </div>
                      <button
                        onClick={() => remove(r.id)}
                        className="text-ink-muted opacity-0 transition-opacity hover:text-red-400 group-hover:opacity-100"
                        aria-label="Delete"
                      >
                        ✕
                      </button>
                    </li>
                  ))}
                  {col.length === 0 && (
                    <li className="px-1 text-xs text-ink-muted">Nothing planned yet.</li>
                  )}
                </ul>
              </section>
            );
          })}
        </div>
      </div>
    </div>
  );
}
