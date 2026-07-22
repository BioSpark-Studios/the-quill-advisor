import { useState } from "react";
import {
  ACCENT_SWATCHES,
  VAULT_ICONS,
  VAULT_TEMPLATES,
  type VaultCustomization,
} from "../../lib/ipc";
import { THEMES } from "../../theme";

const THEME_CHOICES: { id: string | null; label: string }[] = [
  { id: null, label: "App default" },
  ...THEMES,
];

/**
 * Create or customize a vault: name, crest/icon, per-vault theme, accent, and
 * (on create) a starter template of plugins. Vault identity is the bread and
 * butter — this is the control surface for it.
 */
export function VaultDialog({
  mode,
  initialName = "",
  initialCustomization = {},
  onSubmit,
  onClose,
}: {
  mode: "create" | "edit";
  initialName?: string;
  initialCustomization?: VaultCustomization;
  onSubmit: (name: string, customization: VaultCustomization, template?: string[]) => void;
  onClose: () => void;
}) {
  const [name, setName] = useState(initialName);
  const [icon, setIcon] = useState<string | null>(initialCustomization.icon ?? null);
  const [theme, setTheme] = useState<string | null>(initialCustomization.theme ?? null);
  const [accent, setAccent] = useState<string | null>(
    initialCustomization.accent ?? ACCENT_SWATCHES[0].rgb,
  );
  const [templateId, setTemplateId] = useState("advising");

  const canSubmit = mode === "edit" || name.trim().length > 0;

  function submit() {
    if (!canSubmit) return;
    const customization: VaultCustomization = { icon, theme, accent };
    const template =
      mode === "create"
        ? VAULT_TEMPLATES.find((t) => t.id === templateId)?.plugins ?? []
        : undefined;
    onSubmit(name.trim(), customization, template);
  }

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        data-theme={theme ?? undefined}
        className="flex max-h-[90vh] w-full max-w-lg flex-col overflow-hidden rounded-2xl border border-border bg-surface-raised shadow-2xl"
        style={accent ? ({ ["--primary" as string]: accent } as React.CSSProperties) : undefined}
      >
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <h2 className="font-serif text-lg text-ink">
            {mode === "create" ? "New Vault" : "Customize Vault"}
          </h2>
          <button onClick={onClose} className="text-ink-muted hover:text-ink" aria-label="Close">
            ✕
          </button>
        </div>

        <div className="flex-1 space-y-5 overflow-y-auto p-5">
          {/* Live preview */}
          <div className="electric-border flex items-center gap-3 rounded-xl border border-border bg-surface p-4">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/15 text-2xl">
              {icon ?? "🗂️"}
            </div>
            <div className="min-w-0">
              <div className="truncate font-serif text-base text-ink">
                {name.trim() || (mode === "create" ? "Vault name" : initialName)}
              </div>
              <div className="text-xs text-ink-muted">Preview</div>
            </div>
          </div>

          {mode === "create" && (
            <label className="grid gap-1">
              <span className="text-xs font-semibold uppercase tracking-wide text-ink-muted">Name</span>
              <input
                autoFocus
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Counselor or practice name"
                className="rounded-lg border border-border bg-surface px-3 py-2 text-sm text-ink outline-none focus:border-primary"
              />
            </label>
          )}

          {/* Icon / crest */}
          <div className="grid gap-2">
            <span className="text-xs font-semibold uppercase tracking-wide text-ink-muted">Crest</span>
            <div className="flex flex-wrap gap-2">
              {VAULT_ICONS.map((e) => (
                <button
                  key={e}
                  type="button"
                  onClick={() => setIcon(e)}
                  className={`flex h-9 w-9 items-center justify-center rounded-lg border text-xl transition-colors ${
                    icon === e ? "border-primary bg-primary/15" : "border-border hover:border-primary"
                  }`}
                >
                  {e}
                </button>
              ))}
              <button
                type="button"
                onClick={() => setIcon(null)}
                className="rounded-lg border border-dashed border-border px-3 text-xs text-ink-muted hover:border-primary hover:text-primary"
              >
                None
              </button>
            </div>
          </div>

          {/* Accent */}
          <div className="grid gap-2">
            <span className="text-xs font-semibold uppercase tracking-wide text-ink-muted">Accent</span>
            <div className="flex flex-wrap gap-2">
              {ACCENT_SWATCHES.map((s) => (
                <button
                  key={s.id}
                  type="button"
                  onClick={() => setAccent(s.rgb)}
                  title={s.label}
                  className={`h-8 w-8 rounded-full ring-2 ring-offset-2 ring-offset-surface-raised transition-all ${
                    accent === s.rgb ? "ring-ink" : "ring-transparent hover:ring-border"
                  }`}
                  style={{ backgroundColor: `rgb(${s.rgb})` }}
                />
              ))}
            </div>
          </div>

          {/* Theme */}
          <div className="grid gap-2">
            <span className="text-xs font-semibold uppercase tracking-wide text-ink-muted">Theme</span>
            <div className="flex flex-wrap gap-2">
              {THEME_CHOICES.map((t) => (
                <button
                  key={t.label}
                  type="button"
                  onClick={() => setTheme(t.id)}
                  className={`rounded-lg border px-3 py-1.5 text-sm transition-colors ${
                    theme === t.id
                      ? "border-primary bg-primary/15 text-primary"
                      : "border-border text-ink-muted hover:border-primary"
                  }`}
                >
                  {t.label}
                </button>
              ))}
            </div>
          </div>

          {/* Starter template (create only) */}
          {mode === "create" && (
            <div className="grid gap-2">
              <span className="text-xs font-semibold uppercase tracking-wide text-ink-muted">
                Starter template
              </span>
              <div className="grid gap-2">
                {VAULT_TEMPLATES.map((t) => (
                  <button
                    key={t.id}
                    type="button"
                    onClick={() => setTemplateId(t.id)}
                    className={`rounded-xl border p-3 text-left transition-colors ${
                      templateId === t.id
                        ? "border-primary bg-primary/10"
                        : "border-border hover:border-primary"
                    }`}
                  >
                    <div className="text-sm font-medium text-ink">{t.label}</div>
                    <div className="text-xs text-ink-muted">{t.description}</div>
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>

        <div className="flex justify-end gap-2 border-t border-border px-5 py-3">
          <button
            onClick={onClose}
            className="rounded-lg border border-border px-4 py-1.5 text-sm text-ink-muted hover:text-ink"
          >
            Cancel
          </button>
          <button
            onClick={submit}
            disabled={!canSubmit}
            className="rounded-lg bg-primary px-4 py-1.5 text-sm font-medium text-surface-raised disabled:opacity-50"
          >
            {mode === "create" ? "Create vault" : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}
