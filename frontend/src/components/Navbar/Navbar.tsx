import { ThemeButton } from "./ThemeButton/ThemeButton";

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
      <span className="text-2xl font-bold" style={{ color: "var(--primary)" }}>
        Rustsystem
      </span>

      <div className="ml-auto">
        <ThemeButton />
      </div>
    </nav>
  );
}
