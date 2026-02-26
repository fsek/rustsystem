import { Link, createFileRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { Button } from "@/components/Button/Button";
import { Badge } from "@/components/Badge/Badge";

export const Route = createFileRoute("/")({
  component: Landing,
});

// ─── Scroll-reveal hook ───────────────────────────────────────────────────────

function useReveal(threshold = 0.15) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { threshold },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [threshold]);

  return { ref, visible };
}

// ─── Reveal wrapper ────────────────────────────────────────────────────────────

function Reveal({
  children,
  delay = 0,
  direction = "up",
}: {
  children: ReactNode;
  delay?: number;
  direction?: "up" | "left" | "right" | "none";
}) {
  const { ref, visible } = useReveal();

  const translateMap = {
    up: "translateY(32px)",
    left: "translateX(-32px)",
    right: "translateX(32px)",
    none: "none",
  };

  return (
    <div
      ref={ref}
      style={{
        opacity: visible ? 1 : 0,
        transform: visible ? "none" : translateMap[direction],
        transition: `opacity 0.6s ease ${delay}ms, transform 0.6s ease ${delay}ms`,
      }}
    >
      {children}
    </div>
  );
}

// ─── Section divider ──────────────────────────────────────────────────────────

function Divider() {
  return (
    <div
      className="w-full h-px"
      style={{
        background:
          "linear-gradient(90deg, transparent 0%, var(--border) 20%, var(--primary) 50%, var(--border) 80%, transparent 100%)",
      }}
    />
  );
}

// ─── Step card ────────────────────────────────────────────────────────────────

function StepCard({
  number,
  title,
  description,
  icon,
}: {
  number: number;
  title: string;
  description: string;
  icon: ReactNode;
}) {
  return (
    <div
      className="relative flex flex-col gap-4 p-6 rounded-2xl"
      style={{
        background:
          "linear-gradient(135deg, color-mix(in srgb, var(--surface) 95%, var(--primary)) 0%, var(--surface) 100%)",
        border: "1px solid var(--border)",
        boxShadow: "0 2px 8px rgba(0,0,0,0.12), 0 8px 24px rgba(0,0,0,0.08)",
      }}
    >
      {/* Step number bubble */}
      <div
        className="absolute -top-3 -left-3 w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold"
        style={{
          background: "var(--primary)",
          color: "var(--buttonPrimaryText)",
          boxShadow:
            "0 2px 8px color-mix(in srgb, var(--primary) 40%, transparent)",
        }}
      >
        {number}
      </div>

      {/* Icon */}
      <div
        className="w-12 h-12 rounded-xl flex items-center justify-center"
        style={{
          background: "color-mix(in srgb, var(--primary) 12%, var(--surface))",
          color: "var(--primary)",
        }}
      >
        {icon}
      </div>

      <div className="flex flex-col gap-1.5">
        <h3
          className="font-semibold text-lg"
          style={{ color: "var(--textPrimary)" }}
        >
          {title}
        </h3>
        <p
          className="text-sm leading-relaxed"
          style={{ color: "var(--textSecondary)" }}
        >
          {description}
        </p>
      </div>
    </div>
  );
}

// ─── Feature pill ─────────────────────────────────────────────────────────────

function FeaturePill({ label }: { label: string }) {
  return (
    <span
      className="inline-flex items-center gap-2 px-4 py-2 rounded-full text-sm font-medium"
      style={{
        background: "color-mix(in srgb, var(--primary) 10%, var(--surface))",
        border: "1px solid color-mix(in srgb, var(--primary) 25%, transparent)",
        color: "var(--textPrimary)",
      }}
    >
      <span
        className="w-1.5 h-1.5 rounded-full"
        style={{ background: "var(--primary)" }}
      />
      {label}
    </span>
  );
}

// ─── Icons ────────────────────────────────────────────────────────────────────

const IconKey = () => (
  <svg
    width="22"
    height="22"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
  </svg>
);

const IconShield = () => (
  <svg
    width="22"
    height="22"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
  </svg>
);

const IconUsers = () => (
  <svg
    width="22"
    height="22"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
    <circle cx="9" cy="7" r="4" />
    <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
    <path d="M16 3.13a4 4 0 0 1 0 7.75" />
  </svg>
);

const IconCheckCircle = () => (
  <svg
    width="22"
    height="22"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
    <polyline points="22 4 12 14.01 9 11.01" />
  </svg>
);

const IconLock = () => (
  <svg
    width="22"
    height="22"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
    <path d="M7 11V7a5 5 0 0 1 10 0v4" />
  </svg>
);

const IconEye = () => (
  <svg
    width="22"
    height="22"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" />
    <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
    <line x1="1" y1="1" x2="23" y2="23" />
  </svg>
);

// ─── Hero ─────────────────────────────────────────────────────────────────────

function Hero() {
  const [mounted, setMounted] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    const t = setTimeout(() => setMounted(true), 60);
    return () => clearTimeout(t);
  }, []);

  return (
    <section className="relative flex flex-col items-center justify-center min-h-[92vh] px-6 text-center overflow-hidden">
      {/* Ambient glow */}
      <div
        aria-hidden
        className="absolute inset-0 pointer-events-none"
        style={{
          background:
            "radial-gradient(ellipse 70% 50% at 50% 0%, color-mix(in srgb, var(--primary) 14%, transparent) 0%, transparent 70%)",
        }}
      />

      {/* Decorative ring */}
      <div
        aria-hidden
        className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] rounded-full pointer-events-none"
        style={{
          border:
            "1px solid color-mix(in srgb, var(--primary) 12%, transparent)",
          animation: "spin-slow 30s linear infinite",
        }}
      />
      <div
        aria-hidden
        className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[900px] h-[900px] rounded-full pointer-events-none"
        style={{
          border:
            "1px solid color-mix(in srgb, var(--primary) 6%, transparent)",
          animation: "spin-slow 50s linear infinite reverse",
        }}
      />

      <div
        className="relative z-10 flex flex-col items-center gap-8 max-w-4xl"
        style={{
          opacity: mounted ? 1 : 0,
          transform: mounted ? "none" : "translateY(24px)",
          transition: "opacity 0.8s ease, transform 0.8s ease",
        }}
      >
        <Badge size="sm" color="primary" textColor="textPrimary">
          F-sektionen · Lund University
        </Badge>

        <div className="flex flex-col gap-4">
          <h1
            className="text-6xl sm:text-7xl font-black leading-tight tracking-tight"
            style={{ color: "var(--textPrimary)" }}
          >
            Anonymous{" "}
            <span
              style={{
                background: "var(--linearGrad)",
                WebkitBackgroundClip: "text",
                WebkitTextFillColor: "transparent",
                backgroundClip: "text",
              }}
            >
              voting
            </span>
            <br />
            for meetings.
          </h1>
          <p
            className="text-xl max-w-2xl mx-auto leading-relaxed"
            style={{ color: "var(--textSecondary)" }}
          >
            Cryptographically verified, fully anonymous ballots using BLS12-381
            blind signatures. No database. No identity linking.
          </p>
        </div>

        <div className="flex items-center gap-4 flex-wrap justify-center">
          <Button
            size="ml"
            color="buttonPrimary"
            variant="filled"
            onClick={() => navigate({ to: "/create-meeting" })}
          >
            Create Meeting
          </Button>
          <a
            href="https://github.com/fsek/rustsystem"
            target="_blank"
            rel="noreferrer"
          >
            <Button size="ml" color="buttonSecondary" variant="outline">
              View on Github
            </Button>
          </a>
        </div>

        <div className="flex items-center gap-3 flex-wrap justify-center pt-2">
          {["Open source", "In-memory only", "Zero tracking"].map((f) => (
            <FeaturePill key={f} label={f} />
          ))}
        </div>

        <div className="flex items-center gap-1 flex-wrap justify-center text-sm" style={{ color: "var(--textSecondary)" }}>
          <span>Learn more:</span>
          <Link to="/guide" style={{ color: "var(--primary)" }} className="font-medium hover:underline">
            Guide
          </Link>
          <span>·</span>
          <Link to="/encryption" style={{ color: "var(--primary)" }} className="font-medium hover:underline">
            Cryptography
          </Link>
        </div>
      </div>

      {/* Scroll cue */}
      <div
        className="absolute bottom-8 left-1/2 -translate-x-1/2 flex flex-col items-center gap-2"
        style={{
          opacity: mounted ? 0.5 : 0,
          transition: "opacity 1.2s ease 1s",
        }}
      >
        <span
          className="text-xs tracking-widest uppercase"
          style={{ color: "var(--textSecondary)" }}
        >
          Scroll
        </span>
        <div
          className="w-px h-10"
          style={{
            background:
              "linear-gradient(to bottom, var(--primary), transparent)",
          }}
        />
      </div>
    </section>
  );
}

// ─── How it works ─────────────────────────────────────────────────────────────

function HowItWorks() {
  const steps = [
    {
      icon: <IconUsers />,
      title: "Host creates a meeting",
      description:
        "The host opens a meeting and shares QR codes. Voters join and are added to the voter list without any persistent identity records.",
    },
    {
      icon: <IconKey />,
      title: "Voter generates a blind token",
      description:
        "Each voter's browser generates a secret token and a cryptographic commitment. Only the commitment is sent to the signing authority — the token never leaves the browser.",
    },
    {
      icon: <IconShield />,
      title: "Signing authority signs without seeing",
      description:
        "The signing authority (trustauth) confirms the voter is eligible and issues a blind signature — without ever seeing the token. The link between voter and ballot is broken.",
    },
    {
      icon: <IconCheckCircle />,
      title: "Vote is cast anonymously",
      description:
        "The voter submits their ballot directly to the server with a proof derived from the blind signature. The server verifies the proof and marks it as spent — without knowing who voted.",
    },
  ];

  return (
    <section className="px-6 py-24 max-w-6xl mx-auto">
      <Reveal>
        <div className="text-center mb-16">
          <h2
            className="text-4xl font-black mb-4"
            style={{ color: "var(--textPrimary)" }}
          >
            How it works
          </h2>
          <p
            className="text-lg max-w-xl mx-auto"
            style={{ color: "var(--textSecondary)" }}
          >
            A four-step flow where privacy is guaranteed by mathematics, not
            promises.
          </p>
        </div>
      </Reveal>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6">
        {steps.map((step, i) => (
          <Reveal key={step.title} delay={i * 100} direction="up">
            <StepCard number={i + 1} {...step} />
          </Reveal>
        ))}
      </div>
    </section>
  );
}

// ─── Privacy guarantees ───────────────────────────────────────────────────────

function Guarantees() {
  const items = [
    {
      icon: <IconLock />,
      title: "Ballot anonymity",
      body: "A separate signing authority issues blind signatures without seeing the token. The server verifies proofs without knowing who submitted them. Neither service alone can correlate a ballot with a voter.",
    },
    {
      icon: <IconShield />,
      title: "Double-vote prevention",
      body: "Each blind signature is one-time-use. The server marks tokens as spent after submission, making it impossible to vote twice.",
    },
    {
      icon: <IconEye />,
      title: "No persistent storage",
      body: "All meeting state lives in-memory only. When the server restarts, every record is gone; there is no database to breach.",
    },
    {
      icon: <IconKey />,
      title: "Voter-held secrets",
      body: "The random token and blind factor never reach the server. ",
    },
  ];

  return (
    <section
      className="py-24"
      style={{
        background:
          "linear-gradient(180deg, var(--pageBg) 0%, color-mix(in srgb, var(--primary) 4%, var(--pageBg)) 50%, var(--pageBg) 100%)",
      }}
    >
      <div className="max-w-6xl mx-auto px-6">
        <Reveal>
          <div className="text-center mb-16">
            <h2
              className="text-4xl font-black mb-4"
              style={{ color: "var(--textPrimary)" }}
            >
              Privacy by design
            </h2>
            <p
              className="text-lg max-w-xl mx-auto"
              style={{ color: "var(--textSecondary)" }}
            >
              Every layer of the system is built to guarantee anonymity — not as
              a feature, but as a mathematical property.
            </p>
          </div>
        </Reveal>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {items.map((item, i) => (
            <Reveal
              key={item.title}
              delay={i * 80}
              direction={i % 2 === 0 ? "left" : "right"}
            >
              <div
                className="flex gap-5 p-6 rounded-2xl"
                style={{
                  background: "var(--surface)",
                  border: "1px solid var(--border)",
                  boxShadow: "0 2px 8px rgba(0,0,0,0.08)",
                }}
              >
                <div
                  className="shrink-0 w-12 h-12 rounded-xl flex items-center justify-center"
                  style={{
                    background:
                      "color-mix(in srgb, var(--primary) 12%, var(--surface))",
                    color: "var(--primary)",
                  }}
                >
                  {item.icon}
                </div>
                <div className="flex flex-col gap-1.5">
                  <h3
                    className="font-semibold text-base"
                    style={{ color: "var(--textPrimary)" }}
                  >
                    {item.title}
                  </h3>
                  <p
                    className="text-sm leading-relaxed"
                    style={{ color: "var(--textSecondary)" }}
                  >
                    {item.body}
                  </p>
                </div>
              </div>
            </Reveal>
          ))}
        </div>
      </div>
    </section>
  );
}

// ─── CTA ──────────────────────────────────────────────────────────────────────

function CTA() {
  return (
    <section className="py-32 px-6">
      <Reveal>
        <div
          className="relative max-w-3xl mx-auto rounded-3xl p-12 text-center overflow-hidden"
          style={{
            background: "var(--linearGrad)",
            boxShadow:
              "0 8px 40px color-mix(in srgb, var(--primary) 30%, transparent), 0 2px 8px rgba(0,0,0,0.12)",
          }}
        >
          {/* Inner shine */}
          <div
            aria-hidden
            className="absolute inset-0 rounded-3xl pointer-events-none"
            style={{
              background:
                "radial-gradient(ellipse 60% 40% at 50% 0%, rgba(255,255,255,0.18) 0%, transparent 70%)",
            }}
          />

          <div className="relative z-10 flex flex-col items-center gap-6">
            <h2
              className="text-4xl font-black leading-tight"
              style={{ color: "var(--buttonPrimaryText)" }}
            >
              Ready to run your first anonymous vote?
            </h2>
            <p
              className="text-lg max-w-lg"
              style={{
                color:
                  "color-mix(in srgb, var(--buttonPrimaryText) 75%, transparent)",
              }}
            >
              Create your first meeting and be up and running in less than a
              minute.
            </p>
            <Link to="/create-meeting">
              <Button size="l" color="buttonSecondary" variant="filled">
                Create Meeting
              </Button>
            </Link>
          </div>
        </div>
      </Reveal>
    </section>
  );
}

// ─── Footer ───────────────────────────────────────────────────────────────────

function Footer() {
  return (
    <footer
      className="py-10 px-6 text-center flex flex-col items-center gap-3"
      style={{
        borderTop: "1px solid var(--border)",
      }}
    >
      <div className="flex items-center gap-4 text-sm">
        <Link to="/guide" style={{ color: "var(--primary)" }} className="font-medium hover:underline">
          Guide
        </Link>
        <span style={{ color: "var(--border)" }}>|</span>
        <Link to="/encryption" style={{ color: "var(--primary)" }} className="font-medium hover:underline">
          Cryptography
        </Link>
      </div>
      <p className="text-sm" style={{ color: "var(--textSecondary)" }}>
        Built for{" "}
        <span style={{ color: "var(--primary)", fontWeight: 600 }}>
          F-sektionen
        </span>{" "}
        · Lund University · Anonymous voting powered by BLS12-381 blind
        signatures
      </p>
    </footer>
  );
}

// ─── Page ─────────────────────────────────────────────────────────────────────

function Landing() {
  return (
    <div className="min-h-screen" style={{ backgroundColor: "var(--pageBg)" }}>
      <style>{`
        @keyframes spin-slow {
          from { transform: translate(-50%, -50%) rotate(0deg); }
          to   { transform: translate(-50%, -50%) rotate(360deg); }
        }
      `}</style>

      <Hero />
      <Divider />
      <HowItWorks />
      <Divider />
      <Guarantees />
      <Divider />
      <CTA />
      <Footer />
    </div>
  );
}
