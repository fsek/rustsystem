import { Link } from "@tanstack/react-router";

const APP_VERSION = (import.meta.env.APP_VERSION as string | undefined) ?? "dev";

export function Footer() {
  return (
    <footer
      className="py-10 px-6 text-center flex flex-col items-center gap-3"
      style={{ borderTop: "1px solid var(--border)", backgroundColor: "var(--pageBg)" }}
    >
      <div className="flex items-center gap-4 text-sm">
        <Link
          to="/guide"
          style={{ color: "var(--primary)" }}
          className="font-medium hover:underline"
        >
          Guide
        </Link>
        <span style={{ color: "var(--border)" }}>|</span>
        <Link
          to="/encryption"
          style={{ color: "var(--primary)" }}
          className="font-medium hover:underline"
        >
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
      <p className="text-xs" style={{ color: "var(--textSecondary)", opacity: 0.6 }}>
        {APP_VERSION}
      </p>
    </footer>
  );
}
