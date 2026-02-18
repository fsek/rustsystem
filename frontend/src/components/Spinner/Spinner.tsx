import type { Color, Size } from "../types";

export interface SpinnerProps {
  size: Size;
  color: Color;
  className?: string;
}

const SIZE_CLASSES: Record<Size, string> = {
  s: "w-3 h-3",
  sm: "w-4 h-4",
  m: "w-6 h-6",
  ml: "w-8 h-8",
  l: "w-10 h-10",
  xl: "w-14 h-14",
};

const COLOR_VAR: Record<Color, string> = {
  primary: "var(--primary)",
  secondary: "var(--support)",
  accent: "var(--accent)",
};

export function Spinner({ size, color, className = "" }: SpinnerProps) {
  const colorVar = COLOR_VAR[color];
  return (
    <svg
      className={`animate-spin ${SIZE_CLASSES[size]} ${className}`}
      viewBox="0 0 24 24"
      fill="none"
      aria-label="Loading"
    >
      <circle
        cx="12"
        cy="12"
        r="10"
        stroke={colorVar}
        strokeWidth="2.5"
        strokeLinecap="round"
        strokeDasharray="44 16"
        className="opacity-90"
      />
    </svg>
  );
}
