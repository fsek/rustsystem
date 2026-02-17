import type { InputHTMLAttributes } from "react";
import type { Color, Size } from "../types";

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
	size: Size;
	color: Color;
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
	primary: "var(--color-primary)",
	secondary: "var(--color-secondary)",
	accent: "var(--color-accent)",
};

export function Input({ size, color, className = "", style, ...props }: InputProps) {
	return (
		<input
			className={`block outline-none ${SIZE_CLASSES[size]} ${className}`}
			style={{
				border: `1.5px solid ${COLOR_VAR[color]}`,
				backgroundColor: "var(--color-surface)",
				color: "var(--color-primary)",
				...style,
			}}
			{...props}
		/>
	);
}
