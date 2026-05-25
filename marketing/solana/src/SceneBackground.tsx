import { AbsoluteFill, useCurrentFrame } from "remotion";

// Faint static film grain (deterministic feTurbulence) — adds texture and
// kills gradient banding without flicker.
const GRAIN =
  "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='220' height='220'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='2' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E\")";

// Shared backdrop: a dark base with two warm, slowly drifting glows, a vignette
// to keep focus centered, and a touch of grain. Subtle but alive.
export const SceneBackground: React.FC = () => {
  const f = useCurrentFrame();

  // Very slow drift (period ~30-50s) so it breathes rather than distracts.
  const ax = 27 + Math.sin(f * 0.011) * 5;
  const ay = 30 + Math.cos(f * 0.009) * 4;
  const bx = 78 + Math.sin(f * 0.008 + 2.1) * 5;
  const by = 73 + Math.cos(f * 0.0105 + 1.2) * 4;
  const breathe = 0.5 + 0.5 * Math.sin(f * 0.02);

  return (
    <AbsoluteFill style={{ backgroundColor: "#070707" }}>
      <AbsoluteFill
        style={{
          background: `radial-gradient(58% 54% at ${ax}% ${ay}%, rgba(212,165,116,0.15), transparent 68%)`,
          opacity: 0.85 + 0.15 * breathe,
        }}
      />
      <AbsoluteFill
        style={{
          background: `radial-gradient(56% 52% at ${bx}% ${by}%, rgba(150,96,58,0.16), transparent 68%)`,
          opacity: 0.95 - 0.15 * breathe,
        }}
      />
      <AbsoluteFill
        style={{
          background:
            "radial-gradient(125% 125% at 50% 48%, transparent 52%, rgba(0,0,0,0.6) 100%)",
        }}
      />
      <AbsoluteFill
        style={{
          backgroundImage: GRAIN,
          backgroundRepeat: "repeat",
          opacity: 0.04,
          mixBlendMode: "overlay",
        }}
      />
    </AbsoluteFill>
  );
};
