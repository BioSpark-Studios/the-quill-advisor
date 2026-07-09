import { useEffect, useState } from "react";
import { api, STAGES, type VaultCard, type VaultStage } from "../../lib/ipc";
import { ThemeSwitcher } from "../../components/ThemeSwitcher";
import quillMark from "../../assets/logos/crest-the-quill-advisor.png";

/**
 * The landing page after the splash: a kanban board of vault cards grouped by
 * engagement stage. Cards drag between columns; clicking one opens the vault.
 */
export function MasterVault({ onOpenVault }: { onOpenVault: (v: VaultCard) => void }) {
  const [vaults, setVaults] = useState<VaultCard[]>([]);
  const [dragId, setDragId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.listVaults().then((v) => {
      setVaults(v);
      setLoading(false);
    });
  }, []);

  async function drop(stage: VaultStage) {
    if (!dragId) return;
    const id = dragId;
    setDragId(null);
    setVaults((prev) => prev.map((v) => (v.id === id ? { ...v, stage } : v)));
    await api.moveVault(id, stage);
  }

  async function createVault(stage: VaultStage) {
    const name = prompt("Name this vault (e.g. counselor or practice name):");
    if (!name) return;
    const card = await api.createVault(name.trim(), stage);
    setVaults((prev) => [...prev, card]);
  }

  return (
    <div className="min-h-screen">
      <header className="sticky top-0 z-10 flex items-center justify-between border-b border-border bg-surface/80 px-6 py-4 backdrop-blur">
        <div className="flex items-center gap-3">
          <img src={quillMark} alt="" className="h-9 w-9 object-contain" />
          <div>
            <h1 className="font-serif text-xl leading-tight text-ink">Master Vault</h1>
            <p className="text-xs text-ink-muted">Director control · all workspaces</p>
          </div>
        </div>
        <ThemeSwitcher />
      </header>

      <main className="p-6">
        {loading ? (
          <p className="text-ink-muted">Opening the Master Vault…</p>
        ) : (
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            {STAGES.map((col) => {
              const cards = vaults.filter((v) => v.stage === col.id);
              return (
                <section
                  key={col.id}
                  onDragOver={(e) => e.preventDefault()}
                  onDrop={() => drop(col.id)}
                  className="flex flex-col rounded-2xl border border-border bg-surface-raised/40 p-3"
                >
                  <div className="mb-3 flex items-center justify-between px-1">
                    <h2 className="text-sm font-semibold uppercase tracking-wide text-ink-muted">
                      {col.label}
                    </h2>
                    <span className="rounded-full bg-primary/10 px-2 py-0.5 text-xs text-primary">
                      {cards.length}
                    </span>
                  </div>

                  <div className="flex flex-1 flex-col gap-3">
                    {cards.map((v) => (
                      <VaultCardView
                        key={v.id}
                        vault={v}
                        onDragStart={() => setDragId(v.id)}
                        onOpen={() => onOpenVault(v)}
                      />
                    ))}

                    <button
                      type="button"
                      onClick={() => createVault(col.id)}
                      className="rounded-xl border border-dashed border-border py-3 text-sm text-ink-muted transition-colors hover:border-primary hover:text-primary"
                    >
                      ＋ New Vault
                    </button>
                  </div>
                </section>
              );
            })}
          </div>
        )}
      </main>
    </div>
  );
}

function VaultCardView({
  vault,
  onDragStart,
  onOpen,
}: {
  vault: VaultCard;
  onDragStart: () => void;
  onOpen: () => void;
}) {
  return (
    <article
      draggable
      onDragStart={onDragStart}
      onClick={onOpen}
      className="electric-border group cursor-pointer rounded-xl border border-border bg-surface-raised/80 p-4 shadow-sm transition-all hover:-translate-y-0.5 hover:shadow-glow"
    >
      <div className="flex items-start justify-between gap-2">
        <h3 className="font-serif text-base leading-snug text-ink">{vault.name}</h3>
        <span className="h-2.5 w-2.5 shrink-0 rounded-full bg-primary shadow-glow" />
      </div>
      <dl className="mt-3 grid grid-cols-3 gap-2 text-center">
        <Stat label="Students" value={vault.students} />
        <Stat label="Chambers" value={vault.chambers} />
        <Stat label="Hours" value={vault.hourBalance} />
      </dl>
      <p className="mt-3 text-xs text-ink-muted">Last activity: {vault.lastActivity}</p>
    </article>
  );
}

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-lg bg-primary/5 py-1.5">
      <div className="text-lg font-semibold text-primary">{value}</div>
      <div className="text-[10px] uppercase tracking-wide text-ink-muted">{label}</div>
    </div>
  );
}
