/** The AI access point — a pulsing cyan↔emerald orb docked bottom-right. */
export function Orb({ onClick, active }: { onClick: () => void; active: boolean }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label="Open Quantum Quill"
      className="fixed bottom-6 right-6 z-30 h-16 w-16 rounded-full outline-none"
    >
      <span
        className={`absolute inset-0 rounded-full ${active ? "" : "animate-orb-pulse"}`}
        style={{
          background: "radial-gradient(circle at 35% 30%, #34d399, #06b6d4 70%)",
          boxShadow: "0 0 28px -2px rgba(52,211,153,0.7), 0 0 60px -10px rgba(6,182,212,0.6)",
        }}
      />
      <span className="absolute inset-0 flex items-center justify-center text-2xl">✦</span>
    </button>
  );
}
