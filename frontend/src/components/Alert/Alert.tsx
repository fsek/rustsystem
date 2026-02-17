import type { ReactNode } from "react";
import type { Color, Size } from "../types";

export interface AlertProps {
  size: Size;
  color: Color;
  children: ReactNode;
  className?: string;
}

const SIZE_CLASSES: Record<
  Size,
  { text: string; py: string; px: string; gap: string; icon: string }
> = {
  s: {
    text: "text-xs",
    py: "py-1.5",
    px: "px-2",
    gap: "gap-1",
    icon: "text-xs",
  },
  sm: {
    text: "text-xs",
    py: "py-2",
    px: "px-2.5",
    gap: "gap-1.5",
    icon: "text-sm",
  },
  m: {
    text: "text-sm",
    py: "py-2.5",
    px: "px-3",
    gap: "gap-2",
    icon: "text-base",
  },
  ml: {
    text: "text-sm",
    py: "py-3",
    px: "px-4",
    gap: "gap-2.5",
    icon: "text-base",
  },
  l: {
    text: "text-base",
    py: "py-3.5",
    px: "px-5",
    gap: "gap-3",
    icon: "text-lg",
  },
  xl: {
    text: "text-lg",
    py: "py-4",
    px: "px-6",
    gap: "gap-3",
    icon: "text-xl",
  },
};

const COLOR_VAR: Record<Color, string> = {
  primary: "var(--color-primary)",
  secondary: "var(--color-secondary)",
  accent: "var(--color-accent)",
};

export function Alert({ size, color, children, className = "" }: AlertProps) {
  const { text, py, px, gap, icon } = SIZE_CLASSES[size];
  const colorVar = COLOR_VAR[color];
  return (
    <div
      className={`flex items-start ${gap} ${py} ${px} rounded-lg ${className}`}
      style={{
        borderLeft: `4px solid ${colorVar}`,
        backgroundColor: "var(--color-surface)",
      }}
      role="alert"
    >
      <span
        className={`${icon} shrink-0`}
        style={{ color: colorVar }}
        aria-hidden="true"
      >
        ℹ
      </span>
      <div
        className={`${text} font-medium`}
        style={{ color: "var(--color-primary)" }}
      >
        {children}
      </div>
    </div>
  );
}
