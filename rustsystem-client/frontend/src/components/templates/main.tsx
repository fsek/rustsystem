import React from "react";
import '@/colors.css';

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
  <section className="container mx-auto px-4 mt-8">
    <div
      className="flex flex-col lg:flex-row items-center rounded-lg p-8 shadow-xl"
      style={{
        background: `linear-gradient(135deg, var(--gradient-hero-start), var(--gradient-hero-end))`,
      }}
    >
      <div className="lg:w-1/2 lg:pr-8 mb-6 lg:mb-0">
        <h2
          className="text-5xl font-extrabold mb-4 bg-clip-text text-transparent"
          style={{
            backgroundImage: `linear-gradient(90deg, var(--color-main), var(--color-accent2))`,
          }}
        >
          {title}
        </h2>
        <p className="text-lg mb-6 opacity-80">{description}</p>
        {buttonText && onButtonClick && (
          <button
            className="bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-[var(--color-background)] py-3 px-6 rounded-full shadow-lg transform hover:-translate-y-1 transition-all duration-300"
            onClick={onButtonClick}
          >
            {buttonText}
          </button>
        )}
      </div>
    </div>
  </section>
);

export default MainSection;
