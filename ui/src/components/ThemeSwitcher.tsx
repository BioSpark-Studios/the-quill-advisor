import { THEMES, useTheme } from "../theme";

/** Runtime toggle between the app's themes. */
export function ThemeSwitcher() {
  const { theme, setTheme } = useTheme();
  return (
    <div className="flex items-center gap-1 rounded-full border border-border bg-surface-raised/60 p-1">
      {THEMES.map((t) => (
        <button
          key={t.id}
          type="button"
          onClick={() => setTheme(t.id)}
          className={`rounded-full px-3 py-1 text-xs font-medium transition-colors ${
            theme === t.id
              ? "bg-primary text-surface-raised shadow-glow"
              : "text-ink-muted hover:text-ink"
          }`}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
}
