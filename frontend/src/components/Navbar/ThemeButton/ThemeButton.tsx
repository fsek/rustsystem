import { useEffect, useRef, useState } from "react";

export interface Theme {
  name: string;
  primaryColor: string;
  vars: Record<string, string>;
}

export const THEMES: Theme[] = [
  {
    name: "Ocean Light",
    primaryColor: "rgb(9, 99, 126)",
    vars: {
      "--color-primary": "rgb(9, 99, 126)",
      "--color-secondary": "rgb(8, 131, 149)",
      "--color-accent": "rgb(122, 178, 178)",
      "--color-surface": "rgb(235, 244, 246)",
      "--color-background": "white",
      "--color-shadow": "rgba(0, 0, 0, 0.08)",
    },
  },
  {
    name: "Ocean Dark",
    primaryColor: "rgb(44, 116, 179)",
    vars: {
      "--color-primary": "rgb(44, 116, 179)",
      "--color-secondary": "rgb(20, 66, 114)",
      "--color-accent": "rgb(32, 82, 149)",
      "--color-surface": "rgb(10, 38, 71)",
      "--color-background": "rgb(6, 20, 50)",
      "--color-shadow": "rgba(44, 116, 179, 0.25)",
    },
  },
  {
    name: "FSEK Light",
    primaryColor: "rgb(229, 137, 10)",
    vars: {
      "--color-primary": "rgb(229, 137, 10)",
      "--color-secondary": "rgb(157, 92, 13)",
      "--color-accent": "rgb(247, 208, 138)",
      "--color-surface": "rgb(250, 250, 250)",
      "--color-background": "white",
      "--color-shadow": "rgba(0, 0, 0, 0.08)",
    },
  },
  {
    name: "FSEK Dark",
    primaryColor: "rgb(253, 112, 20)",
    vars: {
      "--color-primary": "rgb(253, 112, 20)",
      "--color-secondary": "rgb(57, 62, 70)",
      "--color-accent": "rgb(238, 238, 238)",
      "--color-surface": "rgb(34, 40, 49)",
      "--color-background": "rgb(22, 27, 34)",
      "--color-shadow": "rgba(253, 112, 20, 0.2)",
    },
  },
];

export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  for (const [key, value] of Object.entries(theme.vars)) {
    root.style.setProperty(key, value);
  }
  window.dispatchEvent(new CustomEvent("fsek:theme-change"));
}

export function ThemeButton() {
  const [open, setOpen] = useState(false);
  const [selectedTheme, setSelectedTheme] = useState(() => {
    const saved = localStorage.getItem("fsek:theme");
    return THEMES.find((t) => t.name === saved) ?? THEMES[0];
  });
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Apply the persisted theme on first render.
  useEffect(() => {
    applyTheme(selectedTheme);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!open) return;
    function handleClickOutside(e: MouseEvent) {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(e.target as Node)
      ) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [open]);

  function selectTheme(theme: Theme) {
    setSelectedTheme(theme);
    applyTheme(theme);
    localStorage.setItem("fsek:theme", theme.name);
    setOpen(false);
  }

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        aria-haspopup="listbox"
        aria-expanded={open}
        className="flex items-center gap-1.5 text-sm font-medium px-3 py-2 rounded-lg cursor-pointer transition-colors"
        style={{ color: "var(--color-primary)" }}
        onMouseEnter={(e) =>
          ((e.currentTarget as HTMLElement).style.backgroundColor =
            "var(--color-surface)")
        }
        onMouseLeave={(e) =>
          ((e.currentTarget as HTMLElement).style.backgroundColor =
            "transparent")
        }
      >
        {/* Palette icon */}
        <svg
          className="w-4 h-4"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <circle cx="8" cy="9" r="2.5" />
          <circle cx="16" cy="9" r="2.5" />
          <circle cx="12" cy="17" r="2.5" />
        </svg>
        Theme
        {/* Chevron */}
        <svg
          className={`w-3.5 h-3.5 transition-transform duration-150 ${open ? "rotate-180" : ""}`}
          viewBox="0 0 12 12"
          fill="none"
          aria-hidden="true"
        >
          <path
            d="M2 4l4 4 4-4"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </button>

      {open && (
        <ul
          role="listbox"
          aria-label="Select theme"
          className="absolute right-0 top-full mt-2 w-48 rounded-xl py-1.5"
          style={{
            backgroundColor: "var(--color-surface)",
            boxShadow: "0 4px 12px var(--color-shadow)",
          }}
        >
          {THEMES.map((theme) => {
            const isSelected = theme.name === selectedTheme.name;
            return (
              <li key={theme.name} role="option" aria-selected={isSelected}>
                <button
                  type="button"
                  className="w-full text-left px-4 py-2 text-sm cursor-pointer transition-colors flex items-center gap-2.5"
                  style={{
                    color: "var(--color-primary)",
                    backgroundColor: isSelected
                      ? "var(--color-surface)"
                      : "transparent",
                    fontWeight: isSelected ? 600 : 400,
                  }}
                  onMouseEnter={(e) =>
                    ((e.currentTarget as HTMLElement).style.backgroundColor =
                      "var(--color-surface)")
                  }
                  onMouseLeave={(e) =>
                    ((e.currentTarget as HTMLElement).style.backgroundColor =
                      isSelected ? "var(--color-surface)" : "transparent")
                  }
                  onClick={() => selectTheme(theme)}
                >
                  <span
                    className="w-3.5 h-3.5 rounded-sm shrink-0"
                    style={{ backgroundColor: theme.primaryColor }}
                  />
                  {theme.name}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
