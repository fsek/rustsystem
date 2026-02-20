import type { ButtonHTMLAttributes, ReactNode } from "react";
import type { ButtonColor, Size } from "../types";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  size: Size;
  color: ButtonColor;
  variant?: "filled" | "outline";
  children: ReactNode;
}

const SIZE_CLASSES: Record<Size, string> = {
  s: "text-xs px-2 py-1 rounded font-semibold",
  sm: "text-xs px-2.5 py-1.5 rounded-md font-semibold",
  m: "text-sm px-4 py-2 rounded-lg font-semibold",
  ml: "text-base px-5 py-2.5 rounded-lg font-semibold",
  l: "text-lg px-6 py-3 rounded-xl font-semibold",
  xl: "text-xl px-8 py-3.5 rounded-xl font-semibold",
};

// Filled backgrounds
const BG_VAR: Record<ButtonColor, string> = {
  buttonPrimary: "var(--buttonPrimaryBg)",
  buttonSecondary: "var(--buttonSecondaryBg)",
  linearGrad: "var(--linearGrad)",
  radialGrad: "var(--radialGrad)",
};

// Text color on filled backgrounds
const TEXT_VAR: Record<ButtonColor, string> = {
  buttonPrimary: "var(--buttonPrimaryText)",
  buttonSecondary: "var(--buttonSecondaryText)",
  linearGrad: "var(--buttonPrimaryText)",
  radialGrad: "var(--buttonPrimaryText)",
};

// Color used for the glow shadow (filled) and border/text (outline)
const ACCENT_VAR: Record<ButtonColor, string> = {
  buttonPrimary: "var(--primary)",
  buttonSecondary: "var(--support)",
  linearGrad: "var(--primary)",
  radialGrad: "var(--accent)",
};

export function Button({
  size,
  color,
  variant = "filled",
  children,
  className = "",
  style,
  ...props
}: ButtonProps) {
  const accentVar = ACCENT_VAR[color];
  const colorStyle =
    variant === "filled"
      ? {
          background: BG_VAR[color],
          color: TEXT_VAR[color],
          boxShadow: `0 2px 8px color-mix(in srgb, ${accentVar} 35%, transparent)`,
        }
      : {
          border: `2px solid ${accentVar}`,
          color: accentVar,
          backgroundColor: "transparent",
        };

  return (
    <button
      type="button"
      className={`cursor-pointer transition-all duration-200 hover:-translate-y-0.5 active:translate-y-0 hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50 disabled:translate-y-0 disabled:pointer-events-none ${variant === "outline" ? "hover:bg-[var(--surface)]" : ""} ${SIZE_CLASSES[size]} ${className}`}
      style={{ ...colorStyle, ...style }}
      {...props}
    >
      {children}
    </button>
  );
}
