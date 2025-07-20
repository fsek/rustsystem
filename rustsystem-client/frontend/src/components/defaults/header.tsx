import React from 'react';
import '@/colors.css';

const Header: React.FC = () => {
  return (
    <header className="border-b-2 border-[var(--color-contours)] bg-[var(--color-background)] sticky top-0 z-10 shadow-md">
      <div className="container mx-auto px-4 flex justify-between items-center py-4">
        <h1 className="text-2xl font-bold text-[var(--color-main)] tracking-widest"><a href="/">Rustsystem</a></h1>
        <nav>
          <ul className="flex space-x-6">
            <li><a href="#features" className="hover:text-[var(--color-main)] transition-colors">Features</a></li>
            <li><a href="/about" className="hover:text-[var(--color-main)] transition-colors">About</a></li>
            <li><a href="/contact" className="hover:text-[var(--color-main)] transition-colors">Contact</a></li>
          </ul>
        </nav>
      </div>
    </header>
  );
}

export default Header;
