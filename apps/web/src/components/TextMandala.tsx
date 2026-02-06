interface TextMandalaProps {
  text: string;
}

const RINGS = [
  { radius: 12, chars: 10, speed: 35, dir: 1, color: "ring-0" },
  { radius: 22, chars: 18, speed: 28, dir: -1, color: "ring-1" },
  { radius: 32, chars: 26, speed: 22, dir: 1, color: "ring-2" },
  { radius: 40, chars: 34, speed: 18, dir: -1, color: "ring-3" },
  { radius: 47, chars: 42, speed: 14, dir: 1, color: "ring-4" },
];

export function TextMandala({ text }: TextMandalaProps) {
  const chars = text.replace(/\s/g, "");
  let charIndex = 0;

  return (
    <div className="mandala-container">
      <div className="mandala-glow" />
      {RINGS.map((ring, ri) => (
        <div
          key={ri}
          className={`mandala-ring mandala-ring-${ri}`}
          style={{
            animation: `mandalaRotate ${ring.speed}s linear infinite ${ring.dir === -1 ? "reverse" : "normal"}`,
          }}
        >
          {Array.from({ length: ring.chars }).map((_, ci) => {
            const ch = chars[charIndex % chars.length] || ".";
            charIndex++;
            const angle = (ci / ring.chars) * Math.PI * 2;
            const x = 50 + ring.radius * Math.cos(angle);
            const y = 50 + ring.radius * Math.sin(angle);
            return (
              <span
                key={ci}
                className="mandala-char"
                style={{
                  left: `${x}%`,
                  top: `${y}%`,
                  animationDelay: `${(ci * 0.15) % 2}s`,
                }}
              >
                {ch}
              </span>
            );
          })}
        </div>
      ))}
    </div>
  );
}
