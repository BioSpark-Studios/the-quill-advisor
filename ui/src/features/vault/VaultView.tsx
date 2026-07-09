import { useEffect, useState } from "react";
import { api, type VaultCard } from "../../lib/ipc";
import { Orb } from "../orb/Orb";
import { QuillChat } from "../orb/QuillChat";
import { ThemeSwitcher } from "../../components/ThemeSwitcher";

const BENTO: { title: string; hint: string; span: string }[] = [
  { title: "Pathway Blueprint", hint: "Academic & activity journey", span: "md:col-span-2 md:row-span-2" },
  { title: "Application Timeline Weaver", hint: "Deadlines & milestones", span: "" },
  { title: "Essay Version Control", hint: "Draft history & diffs", span: "" },
  { title: "College Landscape Atlas", hint: "RAG-based explorer", span: "md:col-span-2" },
  { title: "Recommendation Manager", hint: "Letter tracking", span: "" },
  { title: "Billing & Hours", hint: "Local ledger", span: "" },
];

/** Inside a vault: the Magic Bento dashboard, a chamber list with the per-chamber
 *  AI gate, and the Orb access point. */
export function VaultView({ vault, onBack }: { vault: VaultCard; onBack: () => void }) {
  const chambers = Array.from({ length: Math.max(vault.chambers, 3) }, (_, i) => ({
    id: `${vault.id}-chamber-${i + 1}`,
    name: `Student ${i + 1}`,
  }));
  const [activeChamber, setActiveChamber] = useState(chambers[0].id);
  const [aiEnabled, setAiEnabled] = useState(false);
  const [chatOpen, setChatOpen] = useState(false);

  useEffect(() => {
    api.chamberAiEnabled(activeChamber).then(setAiEnabled);
  }, [activeChamber]);

  async function toggleAi() {
    const next = !aiEnabled;
    setAiEnabled(next);
    await api.setChamberAi(activeChamber, next);
  }

  return (
    <div className="min-h-screen">
      <header className="sticky top-0 z-10 flex items-center justify-between border-b border-border bg-surface/80 px-6 py-4 backdrop-blur">
        <div className="flex items-center gap-3">
          <button
            onClick={onBack}
            className="rounded-lg border border-border px-3 py-1.5 text-sm text-ink-muted hover:text-ink"
          >
            ← Master Vault
          </button>
          <div>
            <h1 className="font-serif text-xl leading-tight text-ink">{vault.name}</h1>
            <p className="text-xs text-ink-muted">
              {vault.students} students · {vault.hourBalance} hours remaining
            </p>
          </div>
        </div>
        <ThemeSwitcher />
      </header>

      <main className="grid gap-6 p-6 lg:grid-cols-[240px_1fr]">
        <aside className="rounded-2xl border border-border bg-surface-raised/40 p-3">
          <h2 className="mb-2 px-1 text-sm font-semibold uppercase tracking-wide text-ink-muted">
            Chambers
          </h2>
          <ul className="space-y-1">
            {chambers.map((c) => (
              <li key={c.id}>
                <button
                  onClick={() => setActiveChamber(c.id)}
                  className={`w-full rounded-lg px-3 py-2 text-left text-sm ${
                    activeChamber === c.id
                      ? "bg-primary/15 text-primary"
                      : "text-ink hover:bg-surface-raised"
                  }`}
                >
                  {c.name}
                </button>
              </li>
            ))}
          </ul>

          <div className="mt-4 rounded-xl border border-border p-3">
            <div className="flex items-center justify-between">
              <span className="text-sm text-ink">Quantum Quill</span>
              <button
                onClick={toggleAi}
                role="switch"
                aria-checked={aiEnabled}
                className={`relative h-6 w-11 rounded-full transition-colors ${
                  aiEnabled ? "bg-primary" : "bg-border"
                }`}
              >
                <span
                  className={`absolute top-0.5 h-5 w-5 rounded-full bg-white transition-transform ${
                    aiEnabled ? "translate-x-5" : "translate-x-0.5"
                  }`}
                />
              </button>
            </div>
            <p className="mt-1 text-xs text-ink-muted">
              {aiEnabled ? "AI authorized in this chamber." : "AI is off for this chamber."}
            </p>
          </div>
        </aside>

        {/* Magic Bento dashboard */}
        <section className="grid auto-rows-[130px] grid-cols-1 gap-4 md:grid-cols-3">
          {BENTO.map((tile) => (
            <div
              key={tile.title}
              className={`electric-border flex flex-col justify-between rounded-2xl border border-border bg-surface-raised/70 p-4 transition-shadow hover:shadow-glow ${tile.span}`}
            >
              <h3 className="font-serif text-base text-ink">{tile.title}</h3>
              <p className="text-xs text-ink-muted">{tile.hint}</p>
            </div>
          ))}
        </section>
      </main>

      <Orb onClick={() => setChatOpen((v) => !v)} active={chatOpen} />
      {chatOpen && (
        <QuillChat
          chamberId={activeChamber}
          aiEnabled={aiEnabled}
          onClose={() => setChatOpen(false)}
        />
      )}
    </div>
  );
}
