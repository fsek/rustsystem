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
  primary: "var(--color-primary)",
  secondary: "var(--color-secondary)",
  accent: "var(--color-accent)",
};

export function Spinner({ size, color, className = "" }: SpinnerProps) {
  const colorVar = COLOR_VAR[color];
  return (
    <svg
      className={`animate-spin ${SIZE_CLASSES[size]} ${className}`}
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
      aria-label="Loading"
    >
      <circle
        className="opacity-25"
        cx="12"
        cy="12"
        r="10"
        stroke={colorVar}
        strokeWidth="4"
      />
      <path
        className="opacity-75"
        fill={colorVar}
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
      />
    </svg>
  );
}
