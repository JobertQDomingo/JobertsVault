// The four lights over the canvas, each fading out by 70% of its radius (DESIGN_SYSTEM §2.2).
const LIGHTS = [
  "radial-gradient(40% 55% at 10% 15%, var(--aurora-1) 0%, transparent 70%)",
  "radial-gradient(45% 60% at 90% 10%, var(--aurora-2) 0%, transparent 70%)",
  "radial-gradient(50% 60% at 78% 96%, var(--aurora-3) 0%, transparent 70%)",
  "radial-gradient(40% 50% at 18% 92%, var(--aurora-4) 0%, transparent 70%)",
  "var(--canvas)",
].join(", ");

// Film grain: SVG fractal noise as a data: image, which the content security policy allows.
const GRAIN = `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='.9' numOctaves='2' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E")`;

/** Level 0: painted once behind the app shell, fixed to the window, never moving (§2.2, §10). */
export function AuroraBackground() {
  return (
    <div
      aria-hidden="true"
      className="pointer-events-none fixed inset-0 -z-10"
      style={{ background: LIGHTS }}
    >
      <div
        className="absolute inset-0 mix-blend-overlay"
        style={{ backgroundImage: GRAIN, opacity: "var(--grain)" }}
      />
    </div>
  );
}
