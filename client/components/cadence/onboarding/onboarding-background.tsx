export function OnboardingBackground() {
  return (
    <div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden">
      <div
        className="absolute left-1/2 top-[-30%] h-[520px] w-[820px] -translate-x-1/2 rounded-full opacity-[0.08] blur-[140px]"
        style={{
          background:
            "radial-gradient(closest-side, #d4a574 0%, rgba(212,165,116,0.35) 45%, transparent 75%)",
        }}
      />
    </div>
  )
}
