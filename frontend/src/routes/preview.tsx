import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Alert } from "@/components/Alert/Alert";
import { Badge } from "@/components/Badge/Badge";
import { Button } from "@/components/Button/Button";
import { Card } from "@/components/Card/Card";
import { Input } from "@/components/Input/Input";
import { Spinner } from "@/components/Spinner/Spinner";
import { VoteOption } from "@/components/VoteOption/VoteOption";
import { VoteSection } from "@/components/VoteSection/VoteSection";
import type { VoteSectionHandle } from "@/components/VoteSection/VoteSection";
import type { Color, Size } from "@/components/types";

export const Route = createFileRoute("/preview")({
  component: Preview,
});

// ─── Color / size axes ───────────────────────────────────────────────────────

const COLOR_ROWS: { key: Color; label: string }[] = [
  { key: "primary", label: "Primary" },
  { key: "secondary", label: "Secondary" },
  { key: "accent", label: "Accent" },
];

const SIZES: { key: Size; label: string }[] = [
  { key: "s", label: "Small" },
  { key: "sm", label: "Med-S" },
  { key: "m", label: "Med" },
  { key: "ml", label: "Med-L" },
  { key: "l", label: "Large" },
  { key: "xl", label: "XL" },
];

// ─── Layout helpers ──────────────────────────────────────────────────────────

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mb-16">
      <h2
        className="text-2xl font-bold mb-6 pb-2"
        style={{
          color: "var(--color-primary)",
          borderBottom: "2px solid var(--color-surface)",
        }}
      >
        {title}
      </h2>
      <div className="flex flex-col gap-6">{children}</div>
    </section>
  );
}

function ColorRow({
  label,
  align = "center",
  children,
}: {
  label: string;
  align?: "center" | "start";
  children: ReactNode;
}) {
  return (
    <div
      className={`flex gap-4 ${align === "center" ? "items-center" : "items-start"}`}
    >
      <span
        className="text-xs font-semibold uppercase tracking-wider w-20 shrink-0 pt-1"
        style={{ color: "var(--color-secondary)" }}
      >
        {label}
      </span>
      <div className="flex gap-4 flex-wrap">{children}</div>
    </div>
  );
}

function Sized({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col items-center gap-1">
      {children}
      <span
        className="text-xs text-center block"
        style={{ color: "var(--color-accent)" }}
      >
        {label}
      </span>
    </div>
  );
}

function SizedStart({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col items-start gap-1">
      {children}
      <span
        className="text-xs text-center block"
        style={{ color: "var(--color-accent)" }}
      >
        {label}
      </span>
    </div>
  );
}

// ─── Color palette ───────────────────────────────────────────────────────────

const PALETTE_VARS = [
  { label: "Primary", cssVar: "--color-primary" },
  { label: "Secondary", cssVar: "--color-secondary" },
  { label: "Accent", cssVar: "--color-accent" },
  { label: "Surface", cssVar: "--color-surface" },
  { label: "Background", cssVar: "--color-background" },
];

function readCSSVar(name: string): string {
  return getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();
}

function ColorPalette() {
  // Re-render whenever a theme is applied so live CSS variable values are shown.
  const [, tick] = useState(0);
  useEffect(() => {
    function onThemeChange() {
      tick((n) => n + 1);
    }
    window.addEventListener("fsek:theme-change", onThemeChange);
    return () => window.removeEventListener("fsek:theme-change", onThemeChange);
  }, []);

  return (
    <section className="mb-16">
      <h2
        className="text-2xl font-bold mb-6 pb-2"
        style={{
          color: "var(--color-primary)",
          borderBottom: "2px solid var(--color-surface)",
        }}
      >
        Color Palette
      </h2>
      <div className="flex gap-6 flex-wrap">
        {PALETTE_VARS.map((s) => (
          <div key={s.label} className="flex flex-col items-start gap-1.5">
            <div
              className="w-28 h-28 rounded-2xl"
              style={{
                backgroundColor: `var(${s.cssVar})`,
                boxShadow: "0 1px 4px var(--color-shadow)",
              }}
            />
            <span
              className="text-sm font-semibold"
              style={{ color: "var(--color-primary)" }}
            >
              {s.label}
            </span>
            <span
              className="text-xs font-mono"
              style={{ color: "var(--color-secondary)" }}
            >
              {readCSSVar(s.cssVar)}
            </span>
          </div>
        ))}
      </div>
    </section>
  );
}

// ─── Per-component width hints for the preview grid ─────────────────────────

const INPUT_WIDTHS: Record<Size, string> = {
  s: "w-24",
  sm: "w-28",
  m: "w-32",
  ml: "w-36",
  l: "w-40",
  xl: "w-48",
};

const CARD_WIDTHS: Record<Size, string> = {
  s: "w-24",
  sm: "w-32",
  m: "w-40",
  ml: "w-48",
  l: "w-56",
  xl: "w-64",
};

const VOTE_WIDTHS: Record<Size, string> = {
  s: "w-28",
  sm: "w-32",
  m: "w-40",
  ml: "w-48",
  l: "w-56",
  xl: "w-64",
};

// ─── Component sections ──────────────────────────────────────────────────────

function ButtonsFilled() {
  return (
    <Section title="Buttons — Filled">
      {COLOR_ROWS.map((row) => (
        <ColorRow key={row.key} label={row.label}>
          {SIZES.map((s) => (
            <Sized key={s.key} label={s.label}>
              <Button size={s.key} color={row.key} variant="filled">
                Button
              </Button>
            </Sized>
          ))}
        </ColorRow>
      ))}
    </Section>
  );
}

function ButtonsOutline() {
  return (
    <Section title="Buttons — Outline">
      {COLOR_ROWS.map((row) => (
        <ColorRow key={row.key} label={row.label}>
          {SIZES.map((s) => (
            <Sized key={s.key} label={s.label}>
              <Button size={s.key} color={row.key} variant="outline">
                Button
              </Button>
            </Sized>
          ))}
        </ColorRow>
      ))}
    </Section>
  );
}

function Badges() {
  return (
    <Section title="Badges">
      {COLOR_ROWS.map((row) => (
        <ColorRow key={row.key} label={row.label}>
          {SIZES.map((s) => (
            <Sized key={s.key} label={s.label}>
              <Badge size={s.key} color={row.key}>
                Badge
              </Badge>
            </Sized>
          ))}
        </ColorRow>
      ))}
    </Section>
  );
}

function Inputs() {
  return (
    <Section title="Inputs">
      {COLOR_ROWS.map((row) => (
        <ColorRow key={row.key} label={row.label} align="start">
          {SIZES.map((s) => (
            <SizedStart key={s.key} label={s.label}>
              <Input
                size={s.key}
                color={row.key}
                placeholder="Value..."
                className={INPUT_WIDTHS[s.key]}
                readOnly
              />
            </SizedStart>
          ))}
        </ColorRow>
      ))}
    </Section>
  );
}

function Spinners() {
  return (
    <Section title="Spinners">
      {COLOR_ROWS.map((row) => (
        <ColorRow key={row.key} label={row.label}>
          {SIZES.map((s) => (
            <Sized key={s.key} label={s.label}>
              <Spinner size={s.key} color={row.key} />
            </Sized>
          ))}
        </ColorRow>
      ))}
    </Section>
  );
}

function Alerts() {
  return (
    <Section title="Alerts">
      {COLOR_ROWS.map((row) => (
        <ColorRow key={row.key} label={row.label} align="start">
          {SIZES.map((s) => (
            <SizedStart key={s.key} label={s.label}>
              <Alert size={s.key} color={row.key}>
                Alert message
              </Alert>
            </SizedStart>
          ))}
        </ColorRow>
      ))}
    </Section>
  );
}

function Cards() {
  return (
    <Section title="Cards">
      {COLOR_ROWS.map((row) => (
        <ColorRow key={row.key} label={row.label} align="start">
          {SIZES.map((s) => (
            <SizedStart key={s.key} label={s.label}>
              <Card
                size={s.key}
                color={row.key}
                title="Card title"
                className={CARD_WIDTHS[s.key]}
              >
                Content goes here
              </Card>
            </SizedStart>
          ))}
        </ColorRow>
      ))}
    </Section>
  );
}

// Stateful wrapper so individual VoteOptions are clickable in the preview
function TogglableVoteOption({
  size,
  color,
  className,
}: {
  size: Size;
  color: Color;
  className?: string;
}) {
  const [selected, setSelected] = useState(false);
  return (
    <VoteOption
      size={size}
      color={color}
      label={selected ? "Selected" : "Click me"}
      selected={selected}
      onClick={() => setSelected((s) => !s)}
      className={className}
    />
  );
}

function VoteOptions() {
  return (
    <Section title="Vote Options">
      {COLOR_ROWS.map((row) => (
        <ColorRow key={row.key} label={row.label} align="start">
          {SIZES.map((s) => (
            <SizedStart key={s.key} label={s.label}>
              <TogglableVoteOption
                size={s.key}
                color={row.key}
                className={VOTE_WIDTHS[s.key]}
              />
            </SizedStart>
          ))}
        </ColorRow>
      ))}
    </Section>
  );
}

// Demo wrapper that reads the selection via ref
function VoteSectionDemo({ size, color }: { size: Size; color: Color }) {
  const ref = useRef<VoteSectionHandle>(null);
  const [result, setResult] = useState<string[] | null>(null);
  return (
    <div className="flex flex-col gap-2">
      <VoteSection
        ref={ref}
        size={size}
        color={color}
        options={["In favor", "Against", "Abstain"]}
        className={VOTE_WIDTHS[size]}
      />
      <button
        type="button"
        className="text-xs underline cursor-pointer text-left"
        style={{ color: "var(--color-secondary)" }}
        onClick={() => setResult(ref.current?.getSelected() ?? [])}
      >
        Read selection
      </button>
      {result !== null && (
        <p
          className="text-xs font-mono"
          style={{ color: "var(--color-primary)" }}
        >
          {result.length ? result.join(", ") : "(none)"}
        </p>
      )}
    </div>
  );
}

function VoteSections() {
  return (
    <Section title="Vote Section">
      {COLOR_ROWS.map((row) => (
        <ColorRow key={row.key} label={row.label} align="start">
          {SIZES.map((s) => (
            <SizedStart key={s.key} label={s.label}>
              <VoteSectionDemo size={s.key} color={row.key} />
            </SizedStart>
          ))}
        </ColorRow>
      ))}
    </Section>
  );
}

// ─── Page ────────────────────────────────────────────────────────────────────

function Preview() {
  return (
    <div
      className="min-h-screen p-10"
      style={{ backgroundColor: "var(--color-background)" }}
    >
      <h1
        className="text-4xl font-black mb-1"
        style={{ color: "var(--color-primary)" }}
      >
        Component Preview
      </h1>
      <p className="mb-14 text-sm" style={{ color: "var(--color-secondary)" }}>
        FSEK · Design system — 6 sizes × 3 color variants
      </p>

      <ColorPalette />
      <ButtonsFilled />
      <ButtonsOutline />
      <Badges />
      <Inputs />
      <Spinners />
      <Alerts />
      <Cards />
      <VoteOptions />
      <VoteSections />
    </div>
  );
}
