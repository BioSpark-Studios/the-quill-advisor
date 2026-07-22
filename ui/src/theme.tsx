import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

export type ThemeId = "quill" | "cyan" | "parchment";

export const THEMES: { id: ThemeId; label: string }[] = [
  { id: "quill", label: "Quill Midnight" },
  { id: "cyan", label: "Cyan Holographic" },
  { id: "parchment", label: "Quill Parchment" },
];

const VALID_THEMES = new Set<string>(THEMES.map((t) => t.id));

interface ThemeCtx {
  theme: ThemeId;
  setTheme: (t: ThemeId) => void;
}

const Ctx = createContext<ThemeCtx>({ theme: "quill", setTheme: () => {} });

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<ThemeId>(() => {
    const saved = localStorage.getItem("quill-theme");
    return saved && VALID_THEMES.has(saved) ? (saved as ThemeId) : "quill";
  });

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem("quill-theme", theme);
  }, [theme]);

  return <Ctx.Provider value={{ theme, setTheme }}>{children}</Ctx.Provider>;
}

export function useTheme() {
  return useContext(Ctx);
}
