import '@/colors.css';
import type React from 'react';
import type { MouseEventHandler } from 'react';

interface ButtonProps {
  label: string,
  fn: MouseEventHandler
}

const Button: React.FC<ButtonProps> = ({ label, fn }) => {
  return (
    <button
      type="submit"
      className="bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-[var(--color-background)] py-3 px-6 rounded-full shadow-lg transform hover:-translate-y-1 transition-all duration-300"
      onClick={fn}
    >
      {label}
    </button>
  );
}

export default Button;
