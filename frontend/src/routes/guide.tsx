import { createFileRoute, Link } from "@tanstack/react-router";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { Badge } from "@/components/Badge/Badge";
import { Button } from "@/components/Button/Button";

export const Route = createFileRoute("/guide")({
  component: GuidePage,
});

// ─── Scroll-reveal hook ───────────────────────────────────────────────────────

function useReveal(threshold = 0.12) {
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

function Reveal({
  children,
  delay = 0,
}: {
  children: ReactNode;
  delay?: number;
}) {
  const { ref, visible } = useReveal();
  return (
    <div
      ref={ref}
      style={{
        opacity: visible ? 1 : 0,
        transform: visible ? "none" : "translateY(20px)",
        transition: `opacity 0.5s ease ${delay}ms, transform 0.5s ease ${delay}ms`,
      }}
    >
      {children}
    </div>
  );
}

function Divider() {
  return (
    <div
      className="w-full h-px my-2"
      style={{
        background:
          "linear-gradient(90deg, transparent 0%, var(--border) 20%, var(--primary) 50%, var(--border) 80%, transparent 100%)",
      }}
    />
  );
}

// ─── Step section ─────────────────────────────────────────────────────────────

function StepSection({
  number,
  title,
  children,
}: {
  number: number;
  title: string;
  children: ReactNode;
}) {
  return (
    <Reveal>
      <div
        className="flex gap-6 p-6 rounded-2xl"
        style={{
          background: "var(--surface)",
          border: "1px solid var(--border)",
          boxShadow: "0 2px 8px rgba(0,0,0,0.08)",
        }}
      >
        {/* Step number */}
        <div className="shrink-0 flex flex-col items-center gap-2 pt-0.5">
          <div
            className="w-9 h-9 rounded-full flex items-center justify-center text-sm font-bold"
            style={{
              background: "var(--primary)",
              color: "var(--buttonPrimaryText)",
              boxShadow:
                "0 2px 8px color-mix(in srgb, var(--primary) 40%, transparent)",
            }}
          >
            {number}
          </div>
          <div
            className="w-px flex-1"
            style={{
              background:
                "linear-gradient(to bottom, color-mix(in srgb, var(--primary) 40%, transparent), transparent)",
            }}
          />
        </div>

        {/* Content */}
        <div className="flex flex-col gap-3 pb-2">
          <h3
            className="text-xl font-semibold"
            style={{ color: "var(--textPrimary)" }}
          >
            {title}
          </h3>
          <div
            className="text-base leading-relaxed"
            style={{ color: "var(--textSecondary)" }}
          >
            {children}
          </div>
        </div>
      </div>
    </Reveal>
  );
}

function Note({ children }: { children: ReactNode }) {
  return (
    <p
      className="mt-3 text-sm px-4 py-2.5 rounded-xl"
      style={{
        background: "color-mix(in srgb, var(--primary) 8%, var(--surface))",
        border:
          "1px solid color-mix(in srgb, var(--primary) 20%, transparent)",
        color: "var(--textSecondary)",
      }}
    >
      <span style={{ color: "var(--primary)", fontWeight: 600 }}>Note: </span>
      {children}
    </p>
  );
}

// ─── Page ─────────────────────────────────────────────────────────────────────

function GuidePage() {
  const [mounted, setMounted] = useState(false);
  useEffect(() => {
    const t = setTimeout(() => setMounted(true), 60);
    return () => clearTimeout(t);
  }, []);

  return (
    <div className="min-h-screen" style={{ backgroundColor: "var(--pageBg)" }}>
      {/* Hero */}
      <section className="relative px-6 pt-20 pb-16 text-center overflow-hidden">
        <div
          aria-hidden
          className="absolute inset-0 pointer-events-none"
          style={{
            background:
              "radial-gradient(ellipse 60% 40% at 50% 0%, color-mix(in srgb, var(--primary) 12%, transparent) 0%, transparent 70%)",
          }}
        />
        <div
          className="relative z-10 flex flex-col items-center gap-5 max-w-2xl mx-auto"
          style={{
            opacity: mounted ? 1 : 0,
            transform: mounted ? "none" : "translateY(20px)",
            transition: "opacity 0.7s ease, transform 0.7s ease",
          }}
        >
          <Badge size="sm" color="primary" textColor="textPrimary">
            Quick-start guide
          </Badge>
          <h1
            className="text-5xl font-black leading-tight"
            style={{ color: "var(--textPrimary)" }}
          >
            How to run a{" "}
            <span
              style={{
                background: "var(--linearGrad)",
                WebkitBackgroundClip: "text",
                WebkitTextFillColor: "transparent",
                backgroundClip: "text",
              }}
            >
              meeting
            </span>
          </h1>
          <p
            className="text-lg leading-relaxed"
            style={{ color: "var(--textSecondary)" }}
          >
            Everything you need to run an anonymous vote from start to finish.
          </p>
        </div>
      </section>

      <Divider />

      {/* Steps */}
      <section className="max-w-3xl mx-auto px-6 py-16 flex flex-col gap-4">
        <StepSection number={1} title="Create a meeting">
          <p>
            Navigate to the home page and click <strong style={{ color: "var(--textPrimary)" }}>Create Meeting</strong>. You will be asked for a meeting title, your name, and a password.
          </p>
          <Note>
            The password is used to derive an encryption key for tally files saved on the server. Only someone with the password can decrypt them. Keep it safe — if you lose the password, saved tally files cannot be recovered.
          </Note>
          <p className="mt-3">
            After creation you are logged in as the host and taken to the host dashboard.
          </p>
        </StepSection>

        <StepSection number={2} title="Invite voters">
          <p>
            From the host dashboard, use the <strong style={{ color: "var(--textPrimary)" }}>Add Voter</strong> panel to create an invitation for each participant. Give the voter a name and click <strong style={{ color: "var(--textPrimary)" }}>Add</strong>.
          </p>
          <p className="mt-3">
            Each invitation generates a <strong style={{ color: "var(--textPrimary)" }}>QR code and a unique link</strong>. Share either with the voter. When the voter scans the code or follows the link, they are logged in and appear as active on your dashboard.
          </p>
          <Note>
            Voters who have been created but have never followed their login link are considered <em>unclaimed</em>. When a new vote round starts, all unclaimed voters are automatically removed so no one can join mid-vote.
          </Note>
        </StepSection>

        <StepSection number={3} title="Start a vote round">
          <p>
            When all voters are present, configure the vote round:
          </p>
          <ul className="mt-3 flex flex-col gap-1.5 list-disc list-inside">
            <li>The motion or question to vote on</li>
            <li>The candidates or options (yes/no, list of names, etc.)</li>
            <li>How many options voters may choose</li>
            <li>Whether to shuffle the option order</li>
          </ul>
          <p className="mt-3">
            Press <strong style={{ color: "var(--textPrimary)" }}>Start vote round</strong>. The meeting is now locked — no new voters can join until the round ends.
          </p>
          <Note>
            A <em>blank</em> option is always included automatically. Do not add one yourself.
          </Note>
        </StepSection>

        <StepSection number={4} title="Voters register and cast ballots">
          <p>
            Each voter's browser performs two steps before a ballot is recorded:
          </p>
          <ol className="mt-3 flex flex-col gap-2 list-decimal list-inside">
            <li>
              <strong style={{ color: "var(--textPrimary)" }}>Register</strong> — the voter presses <em>Register to vote</em>. Their browser creates a cryptographic commitment and sends it to the signing authority, which issues a blind signature confirming eligibility without learning the voter's choice.
            </li>
            <li>
              <strong style={{ color: "var(--textPrimary)" }}>Submit</strong> — the voter selects their option(s) and presses <em>Submit</em>. The browser produces a proof from the blind signature and sends it with the vote to the server. The server verifies the proof and marks the signature as spent.
            </li>
          </ol>
          <p className="mt-3">
            The host dashboard shows vote progress in real time.
          </p>
        </StepSection>

        <StepSection number={5} title="Tally the votes">
          <p>
            Once satisfied that everyone has voted, press <strong style={{ color: "var(--textPrimary)" }}>Tally votes</strong>. The server finalises the count, displays the results broken down by option, and saves an encrypted copy of the tally on disk.
          </p>
          <p className="mt-3">
            The tally is only visible on the host dashboard. <strong style={{ color: "var(--textPrimary)" }}>Download the tally</strong> before ending the round — this is the primary way to keep a record.
          </p>
        </StepSection>

        <StepSection number={6} title="End the round">
          <p>
            Press <strong style={{ color: "var(--textPrimary)" }}>End Round</strong> to reset the voting state. The meeting is unlocked and a new round can be started, or voters can be added again.
          </p>
        </StepSection>

        <StepSection number={7} title="Close the meeting">
          <p>
            When the meeting is finished, press <strong style={{ color: "var(--textPrimary)" }}>Close meeting</strong>. All in-memory state is discarded. Encrypted tally files that were written to disk remain on the server.
          </p>
          <p className="mt-3">
            The close-meeting panel includes a <strong style={{ color: "var(--textPrimary)" }}>Download tallies</strong> section. Enter the meeting password to fetch and decrypt every tally file saved during the meeting — they are delivered as a single <code>tallies.json</code>. Decryption happens entirely in your browser; nothing sensitive is sent back to the server.
          </p>
        </StepSection>
      </section>

      <Divider />

      {/* Footer CTA */}
      <section className="py-16 px-6 text-center max-w-2xl mx-auto flex flex-col items-center gap-6">
        <Reveal>
          <p
            className="text-lg"
            style={{ color: "var(--textSecondary)" }}
          >
            Want to understand the cryptography behind the anonymity guarantees?
          </p>
          <div className="mt-4">
            <Link to="/encryption">
              <Button size="m" color="buttonSecondary" variant="outline">
                Read the cryptography overview →
              </Button>
            </Link>
          </div>
        </Reveal>
      </section>
    </div>
  );
}
