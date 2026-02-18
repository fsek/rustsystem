import type { ReactNode } from "react";
import type { Color, Size, TextColor } from "../types";

export interface CardProps {
  size: Size;
  color: Color;
  textColor?: TextColor;
  title?: string;
  children?: ReactNode;
  className?: string;
}

const SIZE_CLASSES: Record<
  Size,
  { p: string; rounded: string; titleCls: string; bodyCls: string }
> = {
  s: {
    p: "p-2",
    rounded: "rounded-lg",
    titleCls: "text-xs font-semibold",
    bodyCls: "text-xs",
  },
  sm: {
    p: "p-3",
    rounded: "rounded-lg",
    titleCls: "text-xs font-semibold",
    bodyCls: "text-xs",
  },
  m: {
    p: "p-4",
    rounded: "rounded-xl",
    titleCls: "text-sm font-semibold",
    bodyCls: "text-sm",
  },
  ml: {
    p: "p-5",
    rounded: "rounded-xl",
    titleCls: "text-sm font-bold",
    bodyCls: "text-sm",
  },
  l: {
    p: "p-6",
    rounded: "rounded-2xl",
    titleCls: "text-base font-bold",
    bodyCls: "text-base",
  },
  xl: {
    p: "p-7",
    rounded: "rounded-2xl",
    titleCls: "text-lg font-bold",
    bodyCls: "text-base",
  },
};

const COLOR_VAR: Record<Color, string> = {
  primary: "var(--primary)",
  secondary: "var(--support)",
  accent: "var(--accent)",
};

export function Card({
  size,
  color,
  textColor = "textPrimary",
  title,
  children,
  className = "",
}: CardProps) {
  const { p, rounded, titleCls, bodyCls } = SIZE_CLASSES[size];
  const colorVar = COLOR_VAR[color];
  return (
    <div
      className={`${p} ${rounded} ${className}`}
      style={{
        background: `linear-gradient(135deg, var(--pageBg) 0%, var(--surface) 100%)`,
        border: `1px solid color-mix(in srgb, ${colorVar} 30%, transparent)`,
        boxShadow: "0 2px 8px rgba(0,0,0,0.3), 0 8px 24px rgba(0,0,0,0.2)",
      }}
    >
      {title && (
        <div className="mb-2">
          <p className={`${titleCls}`} style={{ color: colorVar }}>
            {title}
          </p>
          <div
            className="mt-1.5"
            style={{
              height: "1px",
              backgroundColor: `color-mix(in srgb, ${colorVar} 20%, transparent)`,
            }}
          />
        </div>
      )}
      {children && (
        <div className={bodyCls} style={{ color: `var(--${textColor})` }}>
          {children}
        </div>
      )}
    </div>
  );
}
