import { Link } from "@tanstack/react-router";
import { ThemeButton } from "./ThemeButton/ThemeButton";

const NAV_LINKS = [
  { to: "/guide", label: "Guide" },
  { to: "/encryption", label: "Cryptography" },
] as const;

export function Navbar() {
  return (
    <nav
      className="fixed top-0 left-0 right-0 z-50 flex items-center h-18 px-8"
      style={{
        backgroundColor: "color-mix(in srgb, var(--surface) 80%, transparent)",
        backdropFilter: "blur(12px)",
        WebkitBackdropFilter: "blur(12px)",
        borderBottom: "1px solid var(--border)",
      }}
    >
      <a
        className="text-2xl font-bold"
        style={{ color: "var(--primary)" }}
        href="/"
      >
        Rustsystem
      </a>

      <div className="flex items-center gap-6 ml-8">
        {NAV_LINKS.map(({ to, label }) => (
          <Link
            key={to}
            to={to}
            className="text-sm font-medium transition-colors"
            style={{ color: "var(--textSecondary)" }}
            activeProps={{ style: { color: "var(--primary)" } }}
          >
            {label}
          </Link>
        ))}
      </div>

      <div className="ml-auto">
        <ThemeButton />
      </div>
    </nav>
  );
}
