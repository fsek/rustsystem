import type { InputHTMLAttributes } from "react";
import type React from "react";
import type { Color, Size, TextColor } from "../types";

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  size: Size;
  color: Color;
  textColor?: TextColor;
}

const SIZE_CLASSES: Record<Size, string> = {
  s: "text-xs px-2 py-1 rounded",
  sm: "text-xs px-2.5 py-1.5 rounded-md",
  m: "text-sm px-3 py-2 rounded-lg",
  ml: "text-sm px-3.5 py-2.5 rounded-lg",
  l: "text-base px-4 py-3 rounded-xl",
  xl: "text-lg px-5 py-3.5 rounded-xl",
};

const COLOR_VAR: Record<Color, string> = {
  primary: "var(--primary)",
  secondary: "var(--support)",
  accent: "var(--accent)",
};

export function Input({
  size,
  color,
  textColor = "textPrimary",
  className = "",
  style,
  ...props
}: InputProps) {
  return (
    <input
      className={`fsek-input block transition-all duration-200 ${SIZE_CLASSES[size]} ${className}`}
      style={
        {
          "--input-focus-color": COLOR_VAR[color],
          border: `1.5px solid ${COLOR_VAR[color]}`,
          backgroundColor: "var(--surface)",
          color: `var(--${textColor})`,
          ...style,
        } as React.CSSProperties
      }
      {...props}
    />
  );
}
