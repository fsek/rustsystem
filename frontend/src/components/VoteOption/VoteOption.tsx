import type { Color, Size } from "../types";

export interface VoteOptionProps {
	size: Size;
	color: Color;
	label: string;
	selected?: boolean;
	onClick?: () => void;
	className?: string;
}

const SIZE_CLASSES: Record<Size, { p: string; rounded: string; text: string; checkbox: string; gap: string }> = {
	s: { p: "p-1.5", rounded: "rounded", text: "text-xs", checkbox: "w-3.5 h-3.5", gap: "gap-1.5" },
	sm: { p: "p-2", rounded: "rounded-md", text: "text-xs", checkbox: "w-4 h-4", gap: "gap-2" },
	m: { p: "p-2.5", rounded: "rounded-lg", text: "text-sm", checkbox: "w-5 h-5", gap: "gap-2" },
	ml: { p: "p-3", rounded: "rounded-lg", text: "text-sm", checkbox: "w-5 h-5", gap: "gap-2.5" },
	l: { p: "p-4", rounded: "rounded-xl", text: "text-base", checkbox: "w-6 h-6", gap: "gap-3" },
	xl: { p: "p-5", rounded: "rounded-xl", text: "text-lg", checkbox: "w-7 h-7", gap: "gap-3" },
};

const COLOR_VAR: Record<Color, string> = {
	primary: "var(--color-primary)",
	secondary: "var(--color-secondary)",
	accent: "var(--color-accent)",
};

export function VoteOption({ size, color, label, selected = false, onClick, className = "" }: VoteOptionProps) {
	const { p, rounded, text, checkbox, gap } = SIZE_CLASSES[size];
	const colorVar = COLOR_VAR[color];
	return (
		<button
			type="button"
			onClick={onClick}
			className={`flex items-center ${gap} ${p} ${rounded} w-full cursor-pointer transition-colors text-left ${className}`}
			style={
				selected
					? { border: `2px solid ${colorVar}`, backgroundColor: "var(--color-surface)" }
					: { border: "2px solid var(--color-accent)", backgroundColor: "var(--color-background)" }
			}
			aria-pressed={selected}
		>
			<div
				className={`${checkbox} rounded flex items-center justify-center shrink-0`}
				style={
					selected
						? { backgroundColor: colorVar, border: `2px solid ${colorVar}` }
						: { border: "2px solid var(--color-accent)" }
				}
			>
				{selected && (
					<svg viewBox="0 0 12 12" fill="none" className="w-full h-full p-0.5">
						<path
							d="M2 6.5l2.5 2.5 5.5-5.5"
							stroke="white"
							strokeWidth="2"
							strokeLinecap="round"
							strokeLinejoin="round"
						/>
					</svg>
				)}
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
