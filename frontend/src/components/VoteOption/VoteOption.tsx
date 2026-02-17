import type { Color, Size } from "../types";

export interface VoteOptionProps {
	size: Size;
	color: Color;
	label: string;
	selected?: boolean;
	onClick?: () => void;
	className?: string;
}

const SIZE_CLASSES: Record<
	Size,
	{ p: string; rounded: string; text: string; dot: string; inner: string; gap: string }
> = {
	s: { p: "p-1.5", rounded: "rounded", text: "text-xs", dot: "w-3 h-3", inner: "w-1.5 h-1.5", gap: "gap-1.5" },
	sm: { p: "p-2", rounded: "rounded-md", text: "text-xs", dot: "w-3 h-3", inner: "w-1.5 h-1.5", gap: "gap-2" },
	m: { p: "p-2.5", rounded: "rounded-lg", text: "text-sm", dot: "w-4 h-4", inner: "w-2 h-2", gap: "gap-2" },
	ml: { p: "p-3", rounded: "rounded-lg", text: "text-sm", dot: "w-4 h-4", inner: "w-2 h-2", gap: "gap-2.5" },
	l: { p: "p-4", rounded: "rounded-xl", text: "text-base", dot: "w-5 h-5", inner: "w-2.5 h-2.5", gap: "gap-3" },
	xl: { p: "p-5", rounded: "rounded-xl", text: "text-lg", dot: "w-6 h-6", inner: "w-3 h-3", gap: "gap-3" },
};

const COLOR_VAR: Record<Color, string> = {
	primary: "var(--color-primary)",
	secondary: "var(--color-secondary)",
	accent: "var(--color-accent)",
};

export function VoteOption({ size, color, label, selected = false, onClick, className = "" }: VoteOptionProps) {
	const { p, rounded, text, dot, inner, gap } = SIZE_CLASSES[size];
	const colorVar = COLOR_VAR[color];
	return (
		<button
			type="button"
			onClick={onClick}
			className={`flex items-center ${gap} ${p} ${rounded} w-full cursor-pointer transition-colors text-left ${className}`}
			style={
				selected
					? { border: `2px solid ${colorVar}`, backgroundColor: "var(--color-surface)" }
					: { border: "2px solid var(--color-accent)", backgroundColor: "white" }
			}
			aria-pressed={selected}
		>
			<div
				className={`${dot} rounded-full border-2 flex items-center justify-center shrink-0`}
				style={{ borderColor: selected ? colorVar : "var(--color-accent)" }}
			>
				{selected && <div className={`${inner} rounded-full`} style={{ backgroundColor: colorVar }} />}
			</div>
			<span
				className={`${text} font-medium`}
				style={{ color: "var(--color-primary)", opacity: selected ? 1 : 0.6 }}
			>
				{label}
			</span>
		</button>
	);
}
