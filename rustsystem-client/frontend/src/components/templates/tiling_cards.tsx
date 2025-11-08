import React from "react";
import "@/colors.css";

interface Card {
  title: string;
  content: string;
}

interface TilingCardSectionProps {
  cards: Card[];
}

const TilingCardSection: React.FC<TilingCardSectionProps> = ({ cards }) => {
  const remainder = cards.length % 3;
  const hasRemainder = remainder !== 0;

  return (
    <section className="container mx-auto px-4 mt-12 space-y-8">
      {/* Full Rows (Groups of 3) */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
        {cards.slice(0, cards.length - remainder).map((card, idx) => (
          <CardItem key={idx} title={card.title} content={card.content} />
        ))}
      </div>

      {/* Overflow Row (1 or 2 cards centered) */}
      {hasRemainder && (
        <div
          className={`flex justify-center gap-8 ${
            remainder === 1 ? "justify-center" : "justify-center"
          }`}
        >
          {cards.slice(-remainder).map((card, idx) => (
            <div key={idx} className="w-full max-w-sm">
              <CardItem title={card.title} content={card.content} />
            </div>
          ))}
        </div>
      )}
    </section>
  );
};

const CardItem: React.FC<Card> = ({ title, content }) => (
  <div className="border border-gray-200 rounded-lg p-6 text-left bg-white hover:bg-gray-50 shadow-sm hover:shadow-md transition-all duration-300">
    <h3 className="text-2xl font-bold text-[var(--color-main)] mb-2">
      {title}
    </h3>
    <p className="text-gray-600">{content}</p>
  </div>
);

export default TilingCardSection;
