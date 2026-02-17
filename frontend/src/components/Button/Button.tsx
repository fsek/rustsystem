import type { ButtonHTMLAttributes, ReactNode } from "react";
import type { Color, Size } from "../types";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  size: Size;
  color: Color;
  variant?: "filled" | "outline";
  children: ReactNode;
}

const SIZE_CLASSES: Record<Size, string> = {
  s: "text-xs px-2 py-1 rounded font-medium",
  sm: "text-xs px-2.5 py-1.5 rounded-md font-medium",
  m: "text-sm px-4 py-2 rounded-lg font-medium",
  ml: "text-base px-5 py-2.5 rounded-lg font-medium",
  l: "text-lg px-6 py-3 rounded-xl font-medium",
  xl: "text-xl px-8 py-3.5 rounded-xl font-medium",
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

export function Button({
  size,
  color,
  variant = "filled",
  children,
  className = "",
  style,
  ...props
}: ButtonProps) {
  const colorVar = COLOR_VAR[color];
  const colorStyle =
    variant === "filled"
      ? { backgroundColor: colorVar, color: ON_TEXT[color] }
      : {
          border: `2px solid ${colorVar}`,
          color: colorVar,
          backgroundColor: "transparent",
        };

  return (
    <button
      type="button"
      className={`cursor-pointer transition-opacity hover:opacity-80 ${SIZE_CLASSES[size]} ${className}`}
      style={{ ...colorStyle, ...style }}
      {...props}
    >
      {children}
    </button>
  );
}
