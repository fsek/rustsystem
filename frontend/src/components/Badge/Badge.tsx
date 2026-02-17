import type { ReactNode } from "react";
import type { Color, Size } from "../types";

export interface BadgeProps {
  size: Size;
  color: Color;
  children: ReactNode;
}

const SIZE_CLASSES: Record<Size, string> = {
  s: "text-xs px-1.5 py-0.5 rounded-full font-medium",
  sm: "text-xs px-2 py-0.5 rounded-full font-medium",
  m: "text-sm px-2.5 py-1 rounded-full font-medium",
  ml: "text-sm px-3 py-1 rounded-full font-medium",
  l: "text-base px-3.5 py-1.5 rounded-full font-medium",
  xl: "text-lg px-4 py-1.5 rounded-full font-medium",
};

const COLOR_VAR: Record<Color, string> = {
  primary: "var(--color-primary)",
  secondary: "var(--color-secondary)",
  accent: "var(--color-accent)",
};

const ON_TEXT: Record<Color, string> = {
  primary: "white",
  secondary: "white",
  accent: "var(--color-primary)",
};

export function Badge({ size, color, children }: BadgeProps) {
  return (
    <span
      className={`inline-flex ${SIZE_CLASSES[size]}`}
      style={{ backgroundColor: COLOR_VAR[color], color: ON_TEXT[color] }}
    >
      {children}
    </span>
  );
}
