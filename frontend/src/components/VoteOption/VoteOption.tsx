import type { Color, Size, TextColor } from "../types";

export interface VoteOptionProps {
  size: Size;
  color: Color;
  textColor?: TextColor;
  label: string;
  selected?: boolean;
  onClick?: () => void;
  className?: string;
}

const SIZE_CLASSES: Record<
  Size,
  { p: string; rounded: string; text: string; checkbox: string; gap: string }
> = {
  s: {
    p: "p-1.5",
    rounded: "rounded",
    text: "text-xs",
    checkbox: "w-3.5 h-3.5",
    gap: "gap-1.5",
  },
  sm: {
    p: "p-2",
    rounded: "rounded-md",
    text: "text-xs",
    checkbox: "w-4 h-4",
    gap: "gap-2",
  },
  m: {
    p: "p-2.5",
    rounded: "rounded-lg",
    text: "text-sm",
    checkbox: "w-5 h-5",
    gap: "gap-2",
  },
  ml: {
    p: "p-3",
    rounded: "rounded-lg",
    text: "text-sm",
    checkbox: "w-5 h-5",
    gap: "gap-2.5",
  },
  l: {
    p: "p-4",
    rounded: "rounded-xl",
    text: "text-base",
    checkbox: "w-6 h-6",
    gap: "gap-3",
  },
  xl: {
    p: "p-5",
    rounded: "rounded-xl",
    text: "text-lg",
    checkbox: "w-7 h-7",
    gap: "gap-3",
  },
};

const COLOR_VAR: Record<Color, string> = {
  primary: "var(--primary)",
  secondary: "var(--support)",
  accent: "var(--accent)",
};

export function VoteOption({
  size,
  color,
  textColor = "textPrimary",
  label,
  selected = false,
  onClick,
  className = "",
}: VoteOptionProps) {
  const { p, rounded, text, checkbox, gap } = SIZE_CLASSES[size];
  const colorVar = COLOR_VAR[color];
  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex items-center ${gap} ${p} ${rounded} w-full cursor-pointer transition-all duration-200 text-left hover:scale-[1.01] ${className}`}
      style={
        selected
          ? {
            border: `2px solid ${colorVar}`,
            backgroundColor: "var(--surface)",
            boxShadow: `0 0 0 3px color-mix(in srgb, ${colorVar} 20%, transparent), 0 2px 8px rgba(0,0,0,0.3)`,
          }
          : {
            border: "1px solid var(--border)",
            backgroundColor: "var(--surface)",
            boxShadow: "0 1px 3px rgba(0,0,0,0.3)",
          }
      }
      aria-pressed={selected}
    >
      <div
        className={`${checkbox} rounded flex items-center justify-center shrink-0 transition-all duration-200`}
        style={
          selected
            ? { backgroundColor: colorVar, border: `2px solid ${colorVar}` }
            : { border: "1px solid var(--border)" }
        }
      >
        {selected && (
          <svg viewBox="0 0 12 12" fill="none" className="w-full h-full p-0.5">
            <path
              d="M2 6.5l2.5 2.5 5.5-5.5"
              stroke="white"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        )}
      </div>
      <span
        className={`${text} font-semibold transition-opacity duration-200`}
        style={{ color: `var(--${textColor})`, opacity: selected ? 1 : 0.65 }}
      >
        {label}
      </span>
    </button>
  );
}
