import React from "react";
import "@/colors.css";

const Header: React.FC = () => {
  return (
    <header className="border-b border-gray-200 bg-[var(--color-background)] sticky top-0 z-10 shadow-sm">
      <div className="container mx-auto px-4 flex justify-between items-center py-4">
        <h1 className="text-2xl font-bold text-[var(--color-main)] tracking-wide">
          <a href="/">Rustsystem</a>
        </h1>
      </div>
    </header>
  );
};

export default Header;
