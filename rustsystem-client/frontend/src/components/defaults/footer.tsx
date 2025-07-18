import React from 'react';
import '../../colors.css';

export const Footer: React.FC = () => {
  return (
    <footer className="border-t-2 border-[var(--color-contours)] mt-12 py-6 text-center text-sm opacity-80">
      &copy; {new Date().getFullYear()} F-sektionen at LTH. All rights reserved.
    </footer>
  );
}
