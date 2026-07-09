import { useEffect, useRef, useState } from "react";
import biosparkSeal from "../../assets/logos/seal-biospark.png";
import phantoriCrest from "../../assets/logos/crest-phantori.png";
import quillCrest from "../../assets/logos/crest-the-quill-advisor.png";
import { api } from "../../lib/ipc";

type Phase = "biospark" | "phantori" | "quill" | "leaving";

const HOLD_MS = 1600;
const FADE_MS = 600;

/**
 * Three-stage intro: BioSpark Studios → Phantori → The Quill Advisor.
 * The first two auto-advance; the Quill crest holds until the user clicks it,
 * then cross-fades out and calls `onEnter`. Backend health check runs during
 * the BioSpark stage.
 */
export function Splash({ onEnter }: { onEnter: () => void }) {
  const [phase, setPhase] = useState<Phase>("biospark");
  const [visible, setVisible] = useState(false);
  const healthChecked = useRef(false);

  // Fire the health check once, during the first frame.
  useEffect(() => {
    if (!healthChecked.current) {
      healthChecked.current = true;
      api.healthCheck().catch(() => undefined);
    }
  }, []);

  // Drive the fade-in/hold/fade-out lifecycle for the two auto-advancing stages.
  useEffect(() => {
    if (phase === "quill" || phase === "leaving") return;
    setVisible(false);
    const fadeIn = setTimeout(() => setVisible(true), 40);
    const fadeOut = setTimeout(() => setVisible(false), FADE_MS + HOLD_MS);
    const advance = setTimeout(
      () => setPhase(phase === "biospark" ? "phantori" : "quill"),
      FADE_MS + HOLD_MS + FADE_MS,
    );
    return () => [fadeIn, fadeOut, advance].forEach(clearTimeout);
  }, [phase]);

  // Fade the Quill crest in and leave it up for the click.
  useEffect(() => {
    if (phase !== "quill") return;
    const t = setTimeout(() => setVisible(true), 40);
    return () => clearTimeout(t);
  }, [phase]);

  function enter() {
    if (phase !== "quill") return;
    setVisible(false);
    setPhase("leaving");
    setTimeout(onEnter, FADE_MS);
  }

  const crest =
    phase === "biospark" ? biosparkSeal : phase === "phantori" ? phantoriCrest : quillCrest;
  const caption =
    phase === "biospark"
      ? "BioSpark Studios"
      : phase === "phantori"
        ? "a Phantori production"
        : "The Quill Advisor";
  const clickable = phase === "quill";

  return (
    <div className="cosmic-backdrop fixed inset-0 z-50 flex flex-col items-center justify-center">
      <button
        type="button"
        onClick={enter}
        disabled={!clickable}
        aria-label={clickable ? "Enter The Quill Advisor" : caption}
        className={`group relative flex flex-col items-center outline-none transition-opacity ${
          clickable ? "cursor-pointer" : "cursor-default"
        }`}
        style={{
          opacity: visible ? 1 : 0,
          transitionDuration: `${FADE_MS}ms`,
        }}
      >
        {/* Rotating holographic ring behind the crest */}
        <div className="relative flex h-[320px] w-[320px] items-center justify-center">
          <div
            className="absolute inset-0 rounded-full opacity-60 blur-sm animate-spin-slow"
            style={{
              background:
                "conic-gradient(from 0deg, transparent, rgba(6,182,212,0.6), rgba(52,211,153,0.6), transparent 60%)",
              mask: "radial-gradient(circle, transparent 62%, #000 63%, #000 66%, transparent 67%)",
              WebkitMask:
                "radial-gradient(circle, transparent 62%, #000 63%, #000 66%, transparent 67%)",
            }}
          />
          <img
            src={crest}
            alt={caption}
            draggable={false}
            className={`relative h-[280px] w-[280px] select-none object-contain drop-shadow-[0_0_30px_rgba(52,211,153,0.35)] ${
              clickable ? "transition-transform duration-500 group-hover:scale-105" : ""
            }`}
          />
        </div>

        <p
          className="mt-6 font-serif text-lg tracking-wide text-emerald-200/90"
          style={{ textShadow: "0 0 18px rgba(52,211,153,0.35)" }}
        >
          {caption}
        </p>

        {clickable && (
          <span className="mt-4 animate-pulse text-sm uppercase tracking-[0.25em] text-cyan-300/70">
            click the crest to enter
          </span>
        )}
      </button>
    </div>
  );
}
