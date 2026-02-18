import type { ReactNode } from "react";
import type { Color, Size, TextColor } from "../types";

export interface BadgeProps {
  size: Size;
  color: Color;
  textColor?: TextColor;
  children: ReactNode;
}

const SIZE_CLASSES: Record<Size, string> = {
  s: "text-xs px-1.5 py-0.5 rounded-full font-semibold",
  sm: "text-xs px-2 py-0.5 rounded-full font-semibold",
  m: "text-sm px-2.5 py-1 rounded-full font-semibold",
  ml: "text-sm px-3 py-1 rounded-full font-semibold",
  l: "text-base px-3.5 py-1.5 rounded-full font-semibold",
  xl: "text-lg px-4 py-1.5 rounded-full font-semibold",
};

const COLOR_VAR: Record<Color, string> = {
  primary: "var(--primary)",
  secondary: "var(--support)",
  accent: "var(--accent)",
};

const BG_VAR: Record<Color, string> = {
  primary: "var(--buttonPrimaryBg)",
  secondary: "var(--buttonSecondaryBg)",
  accent: "var(--accent)",
};

const ON_TEXT: Record<Color, string> = {
  primary: "var(--buttonPrimaryText)",
  secondary: "var(--buttonSecondaryText)",
  accent: "var(--pageBg)",
};

export function Badge({ size, color, textColor, children }: BadgeProps) {
  const colorVar = COLOR_VAR[color];
  return (
    <span
      className={`inline-flex ${SIZE_CLASSES[size]}`}
      style={{
        backgroundColor: BG_VAR[color],
        color: textColor ? `var(--${textColor})` : ON_TEXT[color],
        boxShadow: `inset 0 1px 0 rgba(255,255,255,0.2), 0 1px 3px color-mix(in srgb, ${colorVar} 40%, transparent)`,
      }}
    >
      {children}
    </span>
  );
}
