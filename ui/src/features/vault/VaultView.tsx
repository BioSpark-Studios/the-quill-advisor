import { useEffect, useState } from "react";
import {
  api,
  type AvailablePlugin,
  type PluginManifest,
  type VaultCard,
  type VaultComposition,
} from "../../lib/ipc";
import { Orb } from "../orb/Orb";
import { QuillChat } from "../orb/QuillChat";
import { AddonManager } from "./AddonManager";
import { DeclarativePlugin } from "../../plugins/DeclarativePlugin";
import { nativeComponent } from "../../plugins/registry";
import { ThemeSwitcher } from "../../components/ThemeSwitcher";

const SPAN: Record<string, string> = {
  "1x1": "",
  "2x1": "md:col-span-2",
  "3x1": "md:col-span-3",
  "1x2": "md:row-span-2",
  "2x2": "md:col-span-2 md:row-span-2",
  "3x2": "md:col-span-3 md:row-span-2",
};

/** Inside a vault: a dashboard composed entirely from the vault's enabled
 *  plugins, the per-chamber AI gate, the Forge Addon Manager, and the Orb. */
export function VaultView({ vault, onBack }: { vault: VaultCard; onBack: () => void }) {
  const chambers = Array.from({ length: Math.max(vault.chambers, 3) }, (_, i) => ({
    id: `${vault.id}-chamber-${i + 1}`,
    name: `Student ${i + 1}`,
  }));
  const [activeChamber, setActiveChamber] = useState(chambers[0].id);
  const [aiEnabled, setAiEnabled] = useState(false);
  const [chatOpen, setChatOpen] = useState(false);
  const [available, setAvailable] = useState<AvailablePlugin[]>([]);
  const [composition, setComposition] = useState<VaultComposition | null>(null);
  const [openPluginId, setOpenPluginId] = useState<string | null>(null);
  const [managerOpen, setManagerOpen] = useState(false);

  // Load the plugin catalog and this vault's composition; seed a default the
  // first time so the dashboard isn't empty.
  useEffect(() => {
    (async () => {
      const avail = await api.listAvailablePlugins();
      setAvailable(avail);
      let comp = await api.getVaultComposition(vault.id);
      if (comp.plugins.length === 0) {
        const builtins = avail.filter((a) => a.source === "builtin");
        comp = {
          customization: comp.customization ?? {},
          plugins: builtins.map((a, i) => ({
            plugin_id: a.manifest.id,
            settings: null,
            layout: a.manifest.default_layout,
            order: i,
          })),
        };
        await api.setVaultComposition(vault.id, comp);
      }
      setComposition(comp);
    })();
  }, [vault.id]);

  useEffect(() => {
    api.chamberAiEnabled(activeChamber).then(setAiEnabled);
  }, [activeChamber]);

  async function toggleAi() {
    const next = !aiEnabled;
    setAiEnabled(next);
    await api.setChamberAi(activeChamber, next);
  }

  async function updateComposition(next: VaultComposition) {
    setComposition(next);
    await api.setVaultComposition(vault.id, next);
  }

  function manifestFor(id: string): PluginManifest | undefined {
    return available.find((a) => a.manifest.id === id)?.manifest;
  }

  const tiles = (composition?.plugins ?? [])
    .slice()
    .sort((a, b) => a.order - b.order)
    .map((p) => ({ enabled: p, manifest: manifestFor(p.plugin_id) }))
    .filter((t): t is { enabled: (typeof t)["enabled"]; manifest: PluginManifest } => !!t.manifest);

  const openManifest = openPluginId ? manifestFor(openPluginId) : undefined;

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
        <div className="flex items-center gap-2">
          <button
            onClick={() => setManagerOpen(true)}
            className="rounded-lg border border-border px-3 py-1.5 text-sm text-ink hover:border-primary hover:text-primary"
          >
            ⚙ Plugins
          </button>
          <ThemeSwitcher />
        </div>
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

        {/* Magic Bento dashboard, composed from enabled plugins */}
        <section className="grid auto-rows-[130px] grid-cols-1 gap-4 md:grid-cols-3">
          {tiles.map(({ enabled, manifest }) => (
            <button
              key={manifest.id}
              type="button"
              onClick={() => setOpenPluginId(manifest.id)}
              className={`electric-border flex flex-col justify-between rounded-2xl border border-border bg-surface-raised/70 p-4 text-left transition-shadow hover:shadow-glow ${
                SPAN[`${enabled.layout.w}x${enabled.layout.h}`] ?? ""
              }`}
            >
              <div className="text-2xl">{manifest.icon}</div>
              <div>
                <h3 className="font-serif text-base text-ink">{manifest.name}</h3>
                <p className="text-xs text-ink-muted">{manifest.description}</p>
              </div>
            </button>
          ))}

          <button
            type="button"
            onClick={() => setManagerOpen(true)}
            className="flex flex-col items-center justify-center rounded-2xl border border-dashed border-border p-4 text-sm text-ink-muted transition-colors hover:border-primary hover:text-primary"
          >
            ＋ Add plugins
          </button>
        </section>
      </main>

      {/* Launch the opened plugin (native component or declarative renderer) */}
      {openManifest &&
        (openManifest.kind.type === "native"
          ? (() => {
              const Comp = nativeComponent(openManifest.kind.component);
              return Comp ? (
                <Comp
                  manifest={openManifest}
                  vault={vault}
                  chamberId={activeChamber}
                  onClose={() => setOpenPluginId(null)}
                />
              ) : null;
            })()
          : (
              <DeclarativePlugin
                manifest={openManifest}
                vault={vault}
                chamberId={activeChamber}
                onClose={() => setOpenPluginId(null)}
              />
            ))}

      {managerOpen && composition && (
        <AddonManager
          vault={vault}
          available={available}
          composition={composition}
          onChange={updateComposition}
          onCatalogChange={async () => setAvailable(await api.listAvailablePlugins())}
          onClose={() => setManagerOpen(false)}
        />
      )}

      <Orb onClick={() => setChatOpen((v) => !v)} active={chatOpen} />
      {chatOpen && (
        <QuillChat chamberId={activeChamber} aiEnabled={aiEnabled} onClose={() => setChatOpen(false)} />
      )}
    </div>
  );
}
