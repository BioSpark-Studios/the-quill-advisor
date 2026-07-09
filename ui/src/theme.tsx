import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

export type ThemeId = "forest" | "cyan";

export const THEMES: { id: ThemeId; label: string }[] = [
  { id: "forest", label: "Forest Whisper" },
  { id: "cyan", label: "Cyan Holographic" },
];

interface ThemeCtx {
  theme: ThemeId;
  setTheme: (t: ThemeId) => void;
}

const Ctx = createContext<ThemeCtx>({ theme: "forest", setTheme: () => {} });

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<ThemeId>(() => {
    const saved = localStorage.getItem("quill-theme");
    return saved === "cyan" ? "cyan" : "forest";
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
