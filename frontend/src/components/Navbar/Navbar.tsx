import { ThemeButton } from "./ThemeButton/ThemeButton";

export function Navbar() {
  return (
    <nav
      className="fixed top-0 left-0 right-0 z-50 flex items-center h-18 px-8"
      style={{
        backgroundColor: "var(--color-surface)",
        boxShadow: "0 2px 8px var(--color-shadow)",
      }}
    >
      <span
        className="text-2xl font-bold"
        style={{ color: "var(--color-primary)" }}
      >
        Rustsystem
      </span>

      <div className="ml-auto">
        <ThemeButton />
      </div>
    </nav>
  );
}
