/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: ["selector", '[data-theme="quill"], [data-theme="cyan"]'],
  theme: {
    extend: {
      colors: {
        // Theme-driven tokens resolve to CSS variables set per active theme.
        surface: "rgb(var(--surface) / <alpha-value>)",
        "surface-raised": "rgb(var(--surface-raised) / <alpha-value>)",
        ink: "rgb(var(--ink) / <alpha-value>)",
        "ink-muted": "rgb(var(--ink-muted) / <alpha-value>)",
        primary: "rgb(var(--primary) / <alpha-value>)",
        accent: "rgb(var(--accent) / <alpha-value>)",
        border: "rgb(var(--border) / <alpha-value>)",
      },
      fontFamily: {
        sans: ["Inter", "ui-sans-serif", "system-ui", "sans-serif"],
        serif: ["Merriweather", "ui-serif", "Georgia", "serif"],
      },
      boxShadow: {
        glow: "0 0 24px -4px rgb(var(--primary) / 0.55)",
      },
      keyframes: {
        "fade-in": { from: { opacity: "0" }, to: { opacity: "1" } },
        "orb-pulse": {
          "0%,100%": { transform: "scale(1)", opacity: "0.85" },
          "50%": { transform: "scale(1.08)", opacity: "1" },
        },
        spin_slow: { to: { transform: "rotate(360deg)" } },
      },
      animation: {
        "fade-in": "fade-in 0.6s ease forwards",
        "orb-pulse": "orb-pulse 3s ease-in-out infinite",
        "spin-slow": "spin_slow 8s linear infinite",
      },
    },
  },
  plugins: [],
};
