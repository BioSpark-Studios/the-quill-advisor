import { useEffect, useState } from "react";
import { api, type Milestone, type VaultCard } from "../../lib/ipc";

const TODAY = new Date().toISOString().slice(0, 10);

function fmtDate(iso: string | null): string {
  if (!iso) return "No date";
  const d = new Date(`${iso}T00:00:00`);
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

/**
 * Application Timeline Weaver — per-chamber milestones and deadlines, sorted
 * chronologically, backed by the vault's isolated database. Overdue items glow;
 * completed ones dim.
 */
export function TimelineWeaver({
  vault,
  chamberId,
  onClose,
}: {
  vault: VaultCard;
  chamberId: string;
  onClose: () => void;
}) {
  const [items, setItems] = useState<Milestone[]>([]);
  const [title, setTitle] = useState("");
  const [dueAt, setDueAt] = useState("");
  const [busy, setBusy] = useState(false);

  async function refresh() {
    setItems(await api.listMilestones(vault.id, chamberId));
  }

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [chamberId]);

  async function add() {
    if (!title.trim() || busy) return;
    setBusy(true);
    try {
      await api.addMilestone(vault.id, chamberId, title.trim(), dueAt || null);
      setTitle("");
      setDueAt("");
      await refresh();
    } finally {
      setBusy(false);
    }
  }

  async function toggle(m: Milestone) {
    setItems((prev) => prev.map((x) => (x.id === m.id ? { ...x, done: !x.done } : x)));
    await api.setMilestoneDone(vault.id, m.id, !m.done);
  }

  const pending = items.filter((m) => !m.done).length;

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex h-[620px] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border border-border bg-surface-raised shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <div>
            <h2 className="font-serif text-lg text-ink">Application Timeline Weaver</h2>
            <p className="text-xs text-ink-muted">
              {vault.name} · {pending} open
            </p>
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
          className="flex gap-2 border-b border-border p-3"
        >
          <input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="New milestone (e.g. 'Submit UC application')"
            className="flex-1 rounded-lg border border-border bg-surface px-3 py-1.5 text-sm text-ink outline-none focus:border-primary"
          />
          <input
            type="date"
            value={dueAt}
            onChange={(e) => setDueAt(e.target.value)}
            className="rounded-lg border border-border bg-surface px-3 py-1.5 text-sm text-ink outline-none focus:border-primary"
          />
          <button
            type="submit"
            disabled={busy}
            className="rounded-lg bg-primary px-4 py-1.5 text-sm font-medium text-surface-raised disabled:opacity-50"
          >
            Add
          </button>
        </form>

        {/* Timeline */}
        <div className="flex-1 overflow-y-auto p-5">
          {items.length === 0 ? (
            <p className="text-sm text-ink-muted">No milestones yet — add the first deadline above.</p>
          ) : (
            <ol className="relative ml-3 border-l border-border">
              {items.map((m) => {
                const overdue = !m.done && m.dueAt !== null && m.dueAt < TODAY;
                return (
                  <li key={m.id} className="mb-5 ml-6">
                    <span
                      className={`absolute -left-[9px] mt-1 h-4 w-4 rounded-full border-2 ${
                        m.done
                          ? "border-primary bg-primary"
                          : overdue
                            ? "border-red-500 bg-red-500/30"
                            : "border-primary bg-surface-raised"
                      }`}
                    />
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <button
                          onClick={() => toggle(m)}
                          className={`text-left font-serif text-base ${
                            m.done ? "text-ink-muted line-through" : "text-ink"
                          }`}
                        >
                          {m.title}
                        </button>
                        <div
                          className={`text-xs ${overdue ? "font-semibold text-red-400" : "text-ink-muted"}`}
                        >
                          {fmtDate(m.dueAt)}
                          {overdue ? " · overdue" : ""}
                        </div>
                      </div>
                      <button
                        onClick={() => toggle(m)}
                        role="switch"
                        aria-checked={m.done}
                        className={`mt-1 h-5 w-5 shrink-0 rounded border ${
                          m.done ? "border-primary bg-primary text-surface-raised" : "border-border"
                        }`}
                        title={m.done ? "Mark not done" : "Mark done"}
                      >
                        {m.done ? "✓" : ""}
                      </button>
                    </div>
                  </li>
                );
              })}
            </ol>
          )}
        </div>
      </div>
    </div>
  );
}
