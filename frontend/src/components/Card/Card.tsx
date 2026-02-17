import type { ReactNode } from "react";
import type { Color, Size } from "../types";

export interface CardProps {
	size: Size;
	color: Color;
	title?: string;
	children?: ReactNode;
	className?: string;
}

const SIZE_CLASSES: Record<Size, { p: string; rounded: string; titleCls: string; bodyCls: string }> = {
	s: { p: "p-2", rounded: "rounded-lg", titleCls: "text-xs font-semibold", bodyCls: "text-xs" },
	sm: { p: "p-3", rounded: "rounded-lg", titleCls: "text-xs font-semibold", bodyCls: "text-xs" },
	m: { p: "p-4", rounded: "rounded-xl", titleCls: "text-sm font-semibold", bodyCls: "text-sm" },
	ml: { p: "p-5", rounded: "rounded-xl", titleCls: "text-sm font-bold", bodyCls: "text-sm" },
	l: { p: "p-6", rounded: "rounded-2xl", titleCls: "text-base font-bold", bodyCls: "text-base" },
	xl: { p: "p-7", rounded: "rounded-2xl", titleCls: "text-lg font-bold", bodyCls: "text-base" },
};

const COLOR_VAR: Record<Color, string> = {
	primary: "var(--color-primary)",
	secondary: "var(--color-secondary)",
	accent: "var(--color-accent)",
};

export function Card({ size, color, title, children, className = "" }: CardProps) {
	const { p, rounded, titleCls, bodyCls } = SIZE_CLASSES[size];
	const colorVar = COLOR_VAR[color];
	return (
		<div
			className={`${p} ${rounded} bg-white shadow-sm ${className}`}
			style={{ border: `1.5px solid ${colorVar}` }}
		>
			{title && (
				<p className={`${titleCls} mb-1`} style={{ color: colorVar }}>
					{title}
				</p>
			)}
			{children && <div className={bodyCls}>{children}</div>}
		</div>
	);
}
