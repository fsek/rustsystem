import { useState, type ReactNode } from "react";
import type { Color, Size, TextColor } from "../types";

export interface AlertProps {
  size: Size;
  color: Color;
  textColor?: TextColor;
  children: ReactNode;
  className?: string;
  dismissible?: boolean;
  onDismiss?: () => void;
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
  primary: "var(--primary)",
  secondary: "var(--support)",
  accent: "var(--accent)",
};

export function Alert({
  size,
  color,
  textColor = "textPrimary",
  children,
  className = "",
  dismissible = false,
  onDismiss,
}: AlertProps) {
  const [dismissed, setDismissed] = useState(false);
  const { text, py, px, gap, icon } = SIZE_CLASSES[size];
  const colorVar = COLOR_VAR[color];

  if (dismissed) return null;

  return (
    <div
      className={`flex items-start ${gap} ${py} ${px} rounded-lg ${className}`}
      style={{
        borderLeft: `4px solid ${colorVar}`,
        backgroundColor: `color-mix(in srgb, ${colorVar} 8%, var(--surface))`,
        boxShadow: "0 1px 4px rgba(0,0,0,0.3)",
      }}
      role="alert"
    >
      <svg
        className={`${icon} shrink-0`}
        viewBox="0 0 20 20"
        fill="currentColor"
        aria-hidden="true"
        style={{ color: colorVar, width: "1em", height: "1em" }}
      >
        <path
          fillRule="evenodd"
          d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a.75.75 0 000 1.5h.253a.25.25 0 01.244.304l-.459 2.066A1.75 1.75 0 0010.747 15H11a.75.75 0 000-1.5h-.253a.25.25 0 01-.244-.304l.459-2.066A1.75 1.75 0 009.253 9H9z"
          clipRule="evenodd"
        />
      </svg>
      <div
        className={`${text} font-medium flex-1`}
        style={{ color: `var(--${textColor})` }}
      >
        {children}
      </div>
      {dismissible && (
        <button
          type="button"
          onClick={() => {
            setDismissed(true);
            onDismiss?.();
          }}
          aria-label="Dismiss"
          className={`${icon} shrink-0 opacity-60 hover:opacity-100 transition-opacity`}
          style={{ color: colorVar }}
        >
          <svg
            viewBox="0 0 20 20"
            fill="currentColor"
            width="1em"
            height="1em"
            aria-hidden="true"
          >
            <path d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z" />
          </svg>
        </button>
      )}
    </div>
  );
}
