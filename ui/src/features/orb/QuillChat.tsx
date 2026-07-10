import { useState } from "react";
import { api } from "../../lib/ipc";

interface Turn {
  role: "user" | "quill";
  text: string;
  provider?: string;
}

const SUGGESTIONS = ["Brainstorm essay topics", "Tighten my introduction", "Check my authentic voice"];

/**
 * The Quantum Quill chat modal. Scoped to a chamber; if the counselor has not
 * enabled AI for that chamber, Quill is gated and declines to run (mirroring the
 * backend's per-chamber authorization).
 */
export function QuillChat({
  chamberId,
  aiEnabled,
  onClose,
}: {
  chamberId: string;
  aiEnabled: boolean;
  onClose: () => void;
}) {
  const [turns, setTurns] = useState<Turn[]>([
    { role: "quill", text: "Hello! I'm Quantum Quill. How can I help you draft today?" },
  ]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);

  async function send(text: string) {
    const prompt = text.trim();
    if (!prompt || busy) return;
    setInput("");
    setTurns((t) => [...t, { role: "user", text: prompt }]);
    if (!aiEnabled) {
      setTurns((t) => [
        ...t,
        {
          role: "quill",
          text: "AI is turned off for this chamber. Ask the counselor to enable Quantum Quill here first.",
        },
      ]);
      return;
    }
    setBusy(true);
    try {
      const reply = await api.askQuill(chamberId, prompt);
      setTurns((t) => [...t, { role: "quill", text: reply.text, provider: reply.provider }]);
    } catch (e) {
      setTurns((t) => [...t, { role: "quill", text: `Something went wrong: ${String(e)}` }]);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex h-[600px] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border border-border bg-surface-raised shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <div className="flex items-center gap-2">
            <span className="text-lg">✦</span>
            <h2 className="font-serif text-lg text-ink">Quantum Quill</h2>
            {!aiEnabled && (
              <span className="rounded-full bg-amber-500/15 px-2 py-0.5 text-xs text-accent">
                gated in this chamber
              </span>
            )}
          </div>
          <button onClick={onClose} className="text-ink-muted hover:text-ink" aria-label="Close">
            ✕
          </button>
        </div>

        <div className="flex-1 space-y-4 overflow-y-auto p-5">
          {turns.map((t, i) => (
            <div key={i} className={t.role === "user" ? "flex justify-end" : "flex justify-start"}>
              <div
                className={`max-w-[80%] rounded-2xl px-4 py-2.5 text-sm ${
                  t.role === "user"
                    ? "bg-primary text-surface-raised"
                    : "border border-border bg-surface text-ink"
                }`}
              >
                {t.role === "quill" && (
                  <div className="mb-1 text-[10px] font-semibold uppercase tracking-wide text-primary">
                    Quantum Quill{t.provider ? ` · ${t.provider}` : ""}
                  </div>
                )}
                {t.text}
              </div>
            </div>
          ))}
          {busy && <div className="text-sm text-ink-muted">Quill is thinking…</div>}
        </div>

        <div className="border-t border-border p-3">
          <div className="mb-2 flex flex-wrap gap-2">
            {SUGGESTIONS.map((s) => (
              <button
                key={s}
                onClick={() => send(s)}
                className="rounded-full border border-border px-3 py-1 text-xs text-ink-muted hover:border-primary hover:text-primary"
              >
                {s}
              </button>
            ))}
          </div>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              send(input);
            }}
            className="flex gap-2"
          >
            <input
              value={input}
              onChange={(e) => setInput(e.target.value)}
              placeholder="Ask Quantum Quill…"
              className="flex-1 rounded-xl border border-border bg-surface px-4 py-2 text-sm text-ink outline-none focus:border-primary"
            />
            <button
              type="submit"
              disabled={busy}
              className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-surface-raised disabled:opacity-50"
            >
              Send
            </button>
          </form>
        </div>
      </div>
    </div>
  );
}
