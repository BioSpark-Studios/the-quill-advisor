import { useEffect, useMemo, useState } from "react";
import { api, type PluginRecord } from "../../lib/ipc";
import type { PluginProps } from "../../plugins/registry";

const COLLECTION = "threads";
const KINDS = ["Moment", "Value", "Person", "Challenge"] as const;
type Kind = (typeof KINDS)[number];

const KIND_STYLE: Record<Kind, string> = {
  Moment: "bg-sky-500/15 text-sky-400",
  Value: "bg-emerald-500/15 text-emerald-400",
  Person: "bg-violet-500/15 text-violet-400",
  Challenge: "bg-amber-500/15 text-amber-500",
};

interface Angle {
  title: string;
  prompt: string;
}

/**
 * Weave the captured threads into candidate essay angles. Pure, deterministic
 * templating over the student's own material — a Quill agent can expand any
 * angle later, but the loom stands on its own offline.
 */
function weave(byKind: Record<Kind, string[]>): Angle[] {
  const first = (k: Kind) => byKind[k][0];
  const angles: Angle[] = [];
  if (byKind.Moment[0] && byKind.Value[0]) {
    angles.push({
      title: "Moment → Value",
      prompt: `Open on "${first("Moment")}", then trace how it crystallized your belief in ${first("Value")}.`,
    });
  }
  if (byKind.Challenge[0] && byKind.Value[0]) {
    angles.push({
      title: "Growth through challenge",
      prompt: `Frame "${first("Challenge")}" as the turning point that taught you ${first("Value")} — show the before and after.`,
    });
  }
  if (byKind.Person[0] && byKind.Moment[0]) {
    angles.push({
      title: "A person who shaped you",
      prompt: `Center ${first("Person")} in the scene of "${first("Moment")}", then step back to what changed in you.`,
    });
  }
  if (byKind.Challenge[0] && byKind.Person[0]) {
    angles.push({
      title: "Mentor & obstacle",
      prompt: `Contrast "${first("Challenge")}" with a lesson ${first("Person")} gave you — let the tension carry the essay.`,
    });
  }
  return angles;
}

/**
 * Aetherial Narrative Loom — capture the raw threads of a student's story
 * (moments, values, people, challenges), then weave them into candidate essay
 * angles. Threads persist in the vault's capability-gated record store.
 */
export function NarrativeLoom({ manifest, vault, chamberId, onClose }: PluginProps) {
  const [threads, setThreads] = useState<PluginRecord[]>([]);
  const [kind, setKind] = useState<Kind>("Moment");
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);

  async function refresh() {
    setThreads(await api.pluginRecordList(vault.id, chamberId, manifest.id, COLLECTION));
  }

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [chamberId]);

  async function add() {
    if (!text.trim() || busy) return;
    setBusy(true);
    try {
      await api.pluginRecordAdd(vault.id, chamberId, manifest.id, COLLECTION, {
        kind,
        text: text.trim(),
      });
      setText("");
      await refresh();
    } finally {
      setBusy(false);
    }
  }

  async function remove(id: string) {
    setThreads((prev) => prev.filter((r) => r.id !== id));
    await api.pluginRecordDelete(vault.id, id);
  }

  const byKind = useMemo(() => {
    const m: Record<Kind, string[]> = { Moment: [], Value: [], Person: [], Challenge: [] };
    for (const r of threads) {
      const k = String(r.data.kind) as Kind;
      if (m[k]) m[k].push(String(r.data.text));
    }
    return m;
  }, [threads]);

  const angles = useMemo(() => weave(byKind), [byKind]);

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex h-[680px] w-full max-w-4xl flex-col overflow-hidden rounded-2xl border border-border bg-surface-raised shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <div>
            <h2 className="font-serif text-lg text-ink">Aetherial Narrative Loom</h2>
            <p className="text-xs text-ink-muted">{vault.name} · gather threads, weave angles</p>
          </div>
          <button onClick={onClose} className="text-ink-muted hover:text-ink" aria-label="Close">
            ✕
          </button>
        </div>

        <div className="grid flex-1 grid-cols-[1fr_360px] overflow-hidden">
          {/* Threads */}
          <div className="flex flex-col overflow-hidden border-r border-border">
            <form
              onSubmit={(e) => {
                e.preventDefault();
                add();
              }}
              className="flex flex-wrap items-center gap-2 border-b border-border p-3"
            >
              <select
                value={kind}
                onChange={(e) => setKind(e.target.value as Kind)}
                className="rounded-lg border border-border bg-surface px-2 py-1.5 text-sm text-ink outline-none"
              >
                {KINDS.map((k) => (
                  <option key={k} value={k}>
                    {k}
                  </option>
                ))}
              </select>
              <input
                value={text}
                onChange={(e) => setText(e.target.value)}
                placeholder="A thread of the story…"
                className="min-w-[200px] flex-1 rounded-lg border border-border bg-surface px-3 py-1.5 text-sm text-ink outline-none focus:border-primary"
              />
              <button
                type="submit"
                disabled={busy}
                className="rounded-lg bg-primary px-4 py-1.5 text-sm font-medium text-surface-raised disabled:opacity-50"
              >
                Add thread
              </button>
            </form>
            <div className="flex-1 space-y-2 overflow-y-auto p-3">
              {threads.map((r) => (
                <div
                  key={r.id}
                  className="group flex items-start justify-between gap-2 rounded-lg border border-border bg-surface p-2.5"
                >
                  <div className="min-w-0">
                    <span
                      className={`inline-block rounded-full px-2 py-0.5 text-[10px] font-medium ${
                        KIND_STYLE[String(r.data.kind) as Kind] ?? "bg-border/40 text-ink-muted"
                      }`}
                    >
                      {String(r.data.kind)}
                    </span>
                    <div className="mt-1 text-sm text-ink">{String(r.data.text)}</div>
                  </div>
                  <button
                    onClick={() => remove(r.id)}
                    className="text-ink-muted opacity-0 transition-opacity hover:text-red-400 group-hover:opacity-100"
                    aria-label="Delete"
                  >
                    ✕
                  </button>
                </div>
              ))}
              {threads.length === 0 && (
                <p className="text-sm text-ink-muted">
                  Capture a few moments, values, people, and challenges to weave from.
                </p>
              )}
            </div>
          </div>

          {/* Woven angles */}
          <div className="overflow-y-auto bg-surface/40 p-4">
            <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-ink-muted">
              Woven angles
            </h3>
            {angles.length === 0 ? (
              <p className="text-sm text-ink-muted">
                Add at least a moment or challenge plus a value to weave essay angles.
              </p>
            ) : (
              <ul className="space-y-3">
                {angles.map((a, i) => (
                  <li key={i} className="rounded-xl border border-border bg-surface-raised p-3 shadow-sm">
                    <div className="mb-1 font-serif text-sm text-primary">{a.title}</div>
                    <p className="text-sm leading-relaxed text-ink">{a.prompt}</p>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
