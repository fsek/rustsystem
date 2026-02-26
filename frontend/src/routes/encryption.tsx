import { createFileRoute, Link } from "@tanstack/react-router";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { Badge } from "@/components/Badge/Badge";
import { Button } from "@/components/Button/Button";

export const Route = createFileRoute("/encryption")({
  component: EncryptionPage,
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

// ─── Section heading ──────────────────────────────────────────────────────────

function SectionHeading({
  badge,
  title,
  subtitle,
}: {
  badge: string;
  title: string;
  subtitle: string;
}) {
  return (
    <Reveal>
      <div className="flex flex-col gap-3 mb-10">
        <Badge size="sm" color="secondary" textColor="textPrimary">
          {badge}
        </Badge>
        <h2
          className="text-3xl font-black"
          style={{ color: "var(--textPrimary)" }}
        >
          {title}
        </h2>
        <p className="text-base leading-relaxed max-w-2xl" style={{ color: "var(--textSecondary)" }}>
          {subtitle}
        </p>
      </div>
    </Reveal>
  );
}

// ─── Numbered step ────────────────────────────────────────────────────────────

function Step({
  number,
  title,
  children,
}: {
  number: number;
  title: string;
  children: ReactNode;
}) {
  return (
    <Reveal delay={number * 60}>
      <div className="flex gap-4">
        <div className="shrink-0 flex flex-col items-center gap-1 pt-0.5">
          <div
            className="w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold"
            style={{
              background: "color-mix(in srgb, var(--primary) 15%, var(--surface))",
              color: "var(--primary)",
              border: "1px solid color-mix(in srgb, var(--primary) 30%, transparent)",
            }}
          >
            {number}
          </div>
          <div
            className="w-px flex-1 min-h-4"
            style={{
              background:
                "linear-gradient(to bottom, color-mix(in srgb, var(--primary) 25%, transparent), transparent)",
            }}
          />
        </div>
        <div className="flex flex-col gap-1 pb-4">
          <p className="font-semibold text-sm" style={{ color: "var(--textPrimary)" }}>
            {title}
          </p>
          <p className="text-sm leading-relaxed" style={{ color: "var(--textSecondary)" }}>
            {children}
          </p>
        </div>
      </div>
    </Reveal>
  );
}

// ─── Property row ─────────────────────────────────────────────────────────────

function PropertyRow({
  label,
  value,
}: {
  label: string;
  value: string;
}) {
  return (
    <div
      className="flex items-start gap-4 px-4 py-3 rounded-xl"
      style={{
        background: "var(--surface)",
        border: "1px solid var(--border)",
      }}
    >
      <span
        className="shrink-0 text-xs font-semibold uppercase tracking-wider pt-0.5 w-28"
        style={{ color: "var(--primary)" }}
      >
        {label}
      </span>
      <span className="text-sm" style={{ color: "var(--textSecondary)" }}>
        {value}
      </span>
    </div>
  );
}

// ─── Guarantee card ───────────────────────────────────────────────────────────

function GuaranteeCard({
  title,
  body,
}: {
  title: string;
  body: string;
}) {
  return (
    <Reveal>
      <div
        className="p-5 rounded-2xl flex flex-col gap-2"
        style={{
          background: "color-mix(in srgb, var(--primary) 6%, var(--surface))",
          border: "1px solid color-mix(in srgb, var(--primary) 18%, transparent)",
        }}
      >
        <p className="font-semibold text-sm" style={{ color: "var(--textPrimary)" }}>
          {title}
        </p>
        <p className="text-sm leading-relaxed" style={{ color: "var(--textSecondary)" }}>
          {body}
        </p>
      </div>
    </Reveal>
  );
}

// ─── Page ─────────────────────────────────────────────────────────────────────

function EncryptionPage() {
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
            Cryptography
          </Badge>
          <h1
            className="text-5xl font-black leading-tight"
            style={{ color: "var(--textPrimary)" }}
          >
            Privacy by{" "}
            <span
              style={{
                background: "var(--linearGrad)",
                WebkitBackgroundClip: "text",
                WebkitTextFillColor: "transparent",
                backgroundClip: "text",
              }}
            >
              mathematics
            </span>
          </h1>
          <p
            className="text-lg leading-relaxed"
            style={{ color: "var(--textSecondary)" }}
          >
            Rustsystem uses two cryptographic systems: BBS blind signatures for
            anonymous voting, and X25519 ECIES for encrypting tally files.
          </p>
        </div>
      </section>

      <Divider />

      {/* BBS Blind Signatures */}
      <section className="max-w-3xl mx-auto px-6 py-16">
        <SectionHeading
          badge="Voting anonymity"
          title="BBS Blind Signatures over BLS12-381"
          subtitle="The core of Rustsystem's anonymity guarantee. A voter can prove they are authorised to vote without the server ever learning which vote is theirs."
        />

        <div className="mb-8">
          <Reveal>
            <p className="text-sm leading-relaxed mb-6" style={{ color: "var(--textSecondary)" }}>
              BBS signatures are pairing-based signatures defined over the BLS12-381 elliptic curve. The "blind" variant lets a client ask for a signature over a message that is hidden from the signer. The signer cannot see what they are signing, yet the resulting signature is fully verifiable by anyone with the public key.
            </p>
          </Reveal>

          <div className="flex flex-col gap-1">
            <Step number={1} title="Browser generates a secret token and a commitment">
              The voter's browser creates a random secret token and a Pedersen commitment — a hiding, binding commitment to the token. The commitment is sent to the signing authority (trustauth). The token and blind factor remain in the browser and are never transmitted.
            </Step>
            <Step number={2} title="Trustauth verifies eligibility and issues a blind signature">
              Trustauth checks with the server that the voter exists and that voting is active, then confirms the voter has not already registered. It signs the commitment without ever seeing the underlying token, issuing a blind signature.
            </Step>
            <Step number={3} title="Browser derives a proof from the blind signature">
              Using the blind factor, the browser lifts the blind signature into a standard BBS proof of knowledge. This proof can be verified against the trustauth public key by anyone, but it is unlinkable to the original registration request.
            </Step>
            <Step number={4} title="Server verifies the proof without knowing the voter">
              The voter submits their ballot directly to the server along with the proof. The server verifies it against the trustauth public key and marks the underlying signature as spent. It records the vote without any information about who cast it.
            </Step>
          </div>
        </div>

        <Reveal>
          <div className="flex flex-col gap-2 mb-8">
            <p className="text-xs font-semibold uppercase tracking-wider mb-1" style={{ color: "var(--textSecondary)" }}>
              Properties
            </p>
            <PropertyRow label="Curve" value="BLS12-381" />
            <PropertyRow label="Ciphersuite" value="BbsBls12381Sha256" />
            <PropertyRow label="Rust library" value="zkryptium" />
            <PropertyRow label="TS library" value="@noble/curves" />
            <PropertyRow label="Spec" value="draft-irtf-cfrg-bbs-blind-signatures-02" />
          </div>
        </Reveal>

        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <GuaranteeCard
            title="One vote per voter"
            body="Trustauth issues exactly one blind signature per voter per round. The server marks each signature as spent on first use, making double voting impossible."
          />
          <GuaranteeCard
            title="Ballot anonymity"
            body="The server never sees the voter's identity during submission. The proof is unlinkable to the registration request, even with full server logs."
          />
          <GuaranteeCard
            title="Eligibility enforced"
            body="Trustauth checks with the server that the voter is in the meeting and that voting is active before issuing any signature."
          />
        </div>
      </section>

      <Divider />

      {/* X25519 Tally Encryption */}
      <section
        className="py-16"
        style={{
          background:
            "linear-gradient(180deg, var(--pageBg) 0%, color-mix(in srgb, var(--primary) 4%, var(--pageBg)) 50%, var(--pageBg) 100%)",
        }}
      >
        <div className="max-w-3xl mx-auto px-6">
          <SectionHeading
            badge="Tally encryption"
            title="X25519 ECIES with ChaCha20-Poly1305"
            subtitle="Tally files are written to disk encrypted with the host's public key. The server can encrypt but never decrypt — only someone with the meeting password can read the results."
          />

          <div className="flex flex-col gap-1 mb-8">
            <Step number={1} title="Password is stretched into an X25519 private key">
              At meeting creation the host's password is run through PBKDF2-HMAC-SHA256 with a server-side salt to produce a 32-byte seed. This seed is used directly as an X25519 private key. The corresponding public key is stored in the meeting. The private key never reaches the server.
            </Step>
            <Step number={2} title="Server generates an ephemeral keypair and performs ECDH">
              At tally time the server generates a fresh X25519 ephemeral keypair, performs ECDH between the ephemeral private key and the meeting's stored public key, then derives an encryption key with HKDF-SHA256.
            </Step>
            <Step number={3} title="Tally is encrypted with ChaCha20-Poly1305">
              The tally JSON is encrypted using ChaCha20-Poly1305. The output file layout is: ephemeral public key (32 bytes) ‖ nonce (12 bytes) ‖ ciphertext + authentication tag.
            </Step>
            <Step number={4} title="Browser decrypts using the password">
              To recover tallies, the browser re-derives the X25519 private key from the meeting password (same PBKDF2 derivation), downloads the encrypted files, and decrypts them locally. Nothing sensitive is ever sent back to the server.
            </Step>
          </div>

          <Reveal>
            <div className="flex flex-col gap-2 mb-8">
              <p className="text-xs font-semibold uppercase tracking-wider mb-1" style={{ color: "var(--textSecondary)" }}>
                Properties
              </p>
              <PropertyRow label="Key exchange" value="X25519 (ECDH)" />
              <PropertyRow label="KDF" value="PBKDF2-HMAC-SHA256 (key derivation from password)" />
              <PropertyRow label="KDF (ECDH)" value="HKDF-SHA256" />
              <PropertyRow label="Cipher" value="ChaCha20-Poly1305 (authenticated encryption)" />
              <PropertyRow label="Rust libraries" value="x25519-dalek, chacha20poly1305, hkdf" />
            </div>
          </Reveal>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <GuaranteeCard
              title="Server cannot read tallies"
              body="The X25519 private key is derived from the password and never transmitted. The server holds only the public key, so it can encrypt but never decrypt."
            />
            <GuaranteeCard
              title="Filesystem access is not enough"
              body="Anyone with access to the server's disk can see the encrypted files but cannot read them without the meeting password."
            />
          </div>
        </div>
      </section>

      <Divider />

      {/* Footer CTA */}
      <section className="py-16 px-6 text-center max-w-2xl mx-auto flex flex-col items-center gap-6">
        <Reveal>
          <p className="text-lg" style={{ color: "var(--textSecondary)" }}>
            Ready to run an anonymous vote?
          </p>
          <div className="mt-4 flex items-center gap-4 flex-wrap justify-center">
            <Link to="/guide">
              <Button size="m" color="buttonSecondary" variant="outline">
                ← Read the guide
              </Button>
            </Link>
            <Link to="/create-meeting">
              <Button size="m" color="buttonPrimary" variant="filled">
                Create a meeting
              </Button>
            </Link>
          </div>
        </Reveal>
      </section>
    </div>
  );
}
