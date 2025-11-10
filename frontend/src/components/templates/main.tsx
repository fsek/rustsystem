import type React from "react";
import "@/colors.css";

interface MainSectionProps {
	title: string;
	description: React.ReactNode;
	buttonText?: string;
	onButtonClick?: () => void;
}

const MainSection: React.FC<MainSectionProps> = ({
	title,
	description,
	buttonText,
	onButtonClick,
}) => (
	<section className="container mx-auto px-4 py-16">
		<div className="max-w-4xl mx-auto">
			<h2 className="text-4xl font-bold text-[var(--color-contours)] mb-6 tracking-tight">
				{title}
			</h2>
			<div className="text-lg text-gray-600 mb-8 leading-relaxed">
				{description}
			</div>
			{buttonText && onButtonClick && (
				<button
					className="bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-white py-3 px-6 rounded shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100"
					onClick={onButtonClick}
				>
					{buttonText}
				</button>
			)}
		</div>
	</section>
);

export default MainSection;
