import { useEffect, useMemo, useState } from "react";
import { api, type EssaySlot, type EssayVersion } from "../../lib/ipc";
import type { PluginProps } from "../../plugins/registry";

/** Always-available prompts, with the word limits colleges typically set. */
const DEFAULT_ESSAYS: EssaySlot[] = [
  { id: "personal-statement", label: "Personal Statement", wordLimit: 650 },
  { id: "common-app", label: "Common App", wordLimit: 650 },
  { id: "supplement-1", label: "Supplement 1", wordLimit: 250 },
];

function wordCount(text: string): number {
  const trimmed = text.trim();
  return trimmed ? trimmed.split(/\s+/).length : 0;
}

type DiffLine = { type: "same" | "add" | "del"; text: string };

/** Minimal LCS line diff between two texts. */
function lineDiff(a: string, b: string): DiffLine[] {
  const A = a.split("\n");
  const B = b.split("\n");
  const m = A.length;
  const n = B.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
  for (let i = m - 1; i >= 0; i--)
    for (let j = n - 1; j >= 0; j--)
      dp[i][j] = A[i] === B[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
  const out: DiffLine[] = [];
  let i = 0;
  let j = 0;
  while (i < m && j < n) {
    if (A[i] === B[j]) out.push({ type: "same", text: A[i++] }), j++;
    else if (dp[i + 1][j] >= dp[i][j + 1]) out.push({ type: "del", text: A[i++] });
    else out.push({ type: "add", text: B[j++] });
  }
  while (i < m) out.push({ type: "del", text: A[i++] });
  while (j < n) out.push({ type: "add", text: B[j++] });
  return out;
}

/**
 * Git-style essay version control, backed by the vault's isolated database.
 * Commit drafts, browse the revision timeline, and diff a revision against the
 * one before it.
 */
export function EssayVersionControl({ vault, chamberId, onClose }: PluginProps) {
  const [customSlots, setCustomSlots] = useState<EssaySlot[]>([]);
  const essays = useMemo(() => [...DEFAULT_ESSAYS, ...customSlots], [customSlots]);
  const [essayId, setEssayId] = useState(DEFAULT_ESSAYS[0].id);
  const [history, setHistory] = useState<EssayVersion[]>([]);
  const [draft, setDraft] = useState("");
  const [message, setMessage] = useState("");
  const [selected, setSelected] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [addingSlot, setAddingSlot] = useState(false);
  const [newSlotLabel, setNewSlotLabel] = useState("");
  const [newSlotLimit, setNewSlotLimit] = useState("");
  const [feedback, setFeedback] = useState<string | null>(null);
  const [feedbackError, setFeedbackError] = useState<string | null>(null);
  const [coaching, setCoaching] = useState(false);

  async function refresh(id: string) {
    const h = await api.essayHistory(vault.id, id);
    setHistory(h);
    setSelected(h.length ? h.length - 1 : null);
    setDraft(h.length ? h[h.length - 1].body : "");
  }

  useEffect(() => {
    api.getEssaySlots(vault.id).then(setCustomSlots);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [vault.id]);

  useEffect(() => {
    refresh(essayId);
    setMessage("");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [essayId]);

  async function commit() {
    if (!draft.trim() || busy) return;
    setBusy(true);
    try {
      await api.commitEssay(vault.id, chamberId, essayId, message.trim() || "revision", draft);
      setMessage("");
      await refresh(essayId);
    } finally {
      setBusy(false);
    }
  }

  async function addSlot() {
    if (!newSlotLabel.trim() || busy) return;
    const limit = newSlotLimit.trim() ? Number(newSlotLimit) : null;
    setBusy(true);
    try {
      const slot = await api.addEssaySlot(
        vault.id,
        newSlotLabel.trim(),
        limit && Number.isFinite(limit) ? limit : null,
      );
      setCustomSlots((prev) => [...prev, slot]);
      setEssayId(slot.id);
      setNewSlotLabel("");
      setNewSlotLimit("");
      setAddingSlot(false);
    } finally {
      setBusy(false);
    }
  }

  async function removeSlot(id: string) {
    setCustomSlots((prev) => prev.filter((s) => s.id !== id));
    if (essayId === id) setEssayId(DEFAULT_ESSAYS[0].id);
    await api.deleteEssaySlot(vault.id, id);
  }

  const current = essays.find((e) => e.id === essayId);
  const words = wordCount(draft);
  const overLimit = current?.wordLimit != null && words > current.wordLimit;

  async function getFeedback() {
    if (!draft.trim() || coaching) return;
    setCoaching(true);
    setFeedbackError(null);
    try {
      const prompt =
        "Act as an essay voice coach for a college application. Give focused, encouraging " +
        "feedback as 3-5 short bullet points on clarity, impact, and whether the voice sounds " +
        "authentic and specific to this student — don't rewrite it, coach it.\n\n" +
        `Essay draft:\n"""\n${draft}\n"""`;
      const reply = await api.askQuill(chamberId, prompt);
      setFeedback(reply.text);
    } catch {
      setFeedback(null);
      setFeedbackError("Turn on Quantum Quill for this student to get AI feedback.");
    } finally {
      setCoaching(false);
    }
  }

  const diff = useMemo(() => {
    if (selected === null || history.length === 0) return null;
    const cur = history[selected];
    const prev = selected > 0 ? history[selected - 1].body : "";
    return lineDiff(prev, cur.body);
  }, [selected, history]);

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex h-[640px] w-full max-w-5xl flex-col overflow-hidden rounded-2xl border border-border bg-surface-raised shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <div className="flex items-center gap-3">
            <h2 className="font-serif text-lg text-ink">Essay Version Control</h2>
            <select
              value={essayId}
              onChange={(e) => setEssayId(e.target.value)}
              className="rounded-lg border border-border bg-surface px-2 py-1 text-sm text-ink outline-none"
            >
              {essays.map((e) => (
                <option key={e.id} value={e.id}>
                  {e.label}
                </option>
              ))}
            </select>
            {current && customSlots.some((s) => s.id === current.id) && (
              <button
                onClick={() => removeSlot(current.id)}
                className="text-xs text-ink-muted hover:text-red-500"
                title="Remove this custom essay slot"
              >
                remove
              </button>
            )}
            {addingSlot ? (
              <form
                onSubmit={(e) => {
                  e.preventDefault();
                  addSlot();
                }}
                className="flex items-center gap-1"
              >
                <input
                  autoFocus
                  value={newSlotLabel}
                  onChange={(e) => setNewSlotLabel(e.target.value)}
                  placeholder="e.g. 'Why Cornell?'"
                  className="w-32 rounded-lg border border-border bg-surface px-2 py-1 text-xs text-ink outline-none focus:border-primary"
                />
                <input
                  type="number"
                  min="0"
                  value={newSlotLimit}
                  onChange={(e) => setNewSlotLimit(e.target.value)}
                  placeholder="words"
                  className="w-16 rounded-lg border border-border bg-surface px-2 py-1 text-xs text-ink outline-none focus:border-primary"
                />
                <button
                  type="submit"
                  disabled={busy}
                  className="rounded-lg bg-primary px-2 py-1 text-xs font-medium text-surface-raised disabled:opacity-50"
                >
                  Add
                </button>
                <button
                  type="button"
                  onClick={() => setAddingSlot(false)}
                  className="text-xs text-ink-muted hover:text-ink"
                >
                  cancel
                </button>
              </form>
            ) : (
              <button
                onClick={() => setAddingSlot(true)}
                className="text-xs text-ink-muted hover:text-primary"
                title="Add a school-specific supplement"
              >
                + Add essay
              </button>
            )}
            <span className="text-xs text-ink-muted">{vault.name}</span>
          </div>
          <button onClick={onClose} className="text-ink-muted hover:text-ink" aria-label="Close">
            ✕
          </button>
        </div>

        <div className="grid flex-1 grid-cols-[1fr_300px] overflow-hidden">
          {/* Editor + diff */}
          <div className="flex flex-col overflow-hidden border-r border-border">
            <textarea
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              placeholder="Write or paste the essay draft…"
              className="h-1/2 resize-none border-b border-border bg-surface p-4 font-serif text-sm leading-relaxed text-ink outline-none"
            />
            <div className="flex items-center justify-between border-b border-border px-4 py-1.5">
              <span className={`text-xs ${overLimit ? "font-semibold text-red-500" : "text-ink-muted"}`}>
                {words} word{words === 1 ? "" : "s"}
                {current?.wordLimit != null ? ` / ${current.wordLimit}` : ""}
                {overLimit ? " · over limit" : ""}
              </span>
            </div>
            <div className="flex items-center gap-2 border-b border-border p-3">
              <input
                value={message}
                onChange={(e) => setMessage(e.target.value)}
                placeholder="Commit message (e.g. 'tighten the intro')"
                className="flex-1 rounded-lg border border-border bg-surface px-3 py-1.5 text-sm text-ink outline-none focus:border-primary"
              />
              <button
                onClick={getFeedback}
                disabled={coaching || !draft.trim()}
                className="rounded-lg border border-primary px-3 py-1.5 text-sm font-medium text-primary disabled:opacity-50"
                title="Get AI feedback on this draft"
              >
                {coaching ? "Coaching…" : "🪄 Get feedback"}
              </button>
              <button
                onClick={commit}
                disabled={busy}
                className="rounded-lg bg-primary px-4 py-1.5 text-sm font-medium text-surface-raised disabled:opacity-50"
              >
                Commit revision
              </button>
            </div>
            {(feedback || feedbackError) && (
              <div className="border-b border-border bg-primary/5 p-4">
                <div className="mb-1 flex items-center justify-between">
                  <h3 className="text-xs font-semibold uppercase tracking-wide text-primary">
                    Essay Voice Coach
                  </h3>
                  <button
                    onClick={() => {
                      setFeedback(null);
                      setFeedbackError(null);
                    }}
                    className="text-ink-muted hover:text-ink"
                    aria-label="Dismiss feedback"
                  >
                    ✕
                  </button>
                </div>
                {feedbackError ? (
                  <p className="text-sm text-ink-muted">{feedbackError}</p>
                ) : (
                  <p className="whitespace-pre-wrap text-sm leading-relaxed text-ink">{feedback}</p>
                )}
              </div>
            )}
            <div className="flex-1 overflow-y-auto p-4">
              <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-ink-muted">
                {selected !== null && history[selected]
                  ? `Changes in revision #${history[selected].seq}`
                  : "Diff"}
              </h3>
              {diff ? (
                <pre className="whitespace-pre-wrap font-mono text-xs leading-relaxed">
                  {diff.map((l, k) => (
                    <div
                      key={k}
                      className={
                        l.type === "add"
                          ? "bg-emerald-500/15 text-emerald-500"
                          : l.type === "del"
                            ? "bg-red-500/15 text-red-400"
                            : "text-ink-muted"
                      }
                    >
                      {l.type === "add" ? "+ " : l.type === "del" ? "- " : "  "}
                      {l.text || " "}
                    </div>
                  ))}
                </pre>
              ) : (
                <p className="text-sm text-ink-muted">Commit a revision to start tracking history.</p>
              )}
            </div>
          </div>

          {/* Revision timeline */}
          <div className="overflow-y-auto p-3">
            <h3 className="mb-2 px-1 text-xs font-semibold uppercase tracking-wide text-ink-muted">
              History ({history.length})
            </h3>
            <ul className="space-y-2">
              {[...history].reverse().map((v) => {
                const idx = history.indexOf(v);
                return (
                  <li key={v.id}>
                    <button
                      onClick={() => {
                        setSelected(idx);
                        setDraft(v.body);
                      }}
                      className={`w-full rounded-lg border px-3 py-2 text-left ${
                        selected === idx
                          ? "border-primary bg-primary/10"
                          : "border-border hover:border-primary/50"
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-mono text-xs text-primary">#{v.seq}</span>
                      </div>
                      <div className="truncate text-sm text-ink">{v.message}</div>
                    </button>
                  </li>
                );
              })}
              {history.length === 0 && (
                <li className="px-1 text-sm text-ink-muted">No revisions yet.</li>
              )}
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}
