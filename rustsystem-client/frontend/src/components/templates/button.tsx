import "@/colors.css";
import type React from "react";
import type { MouseEventHandler } from "react";

interface ButtonProps {
  label: string;
  fn: MouseEventHandler;
}

const Button: React.FC<ButtonProps> = ({ label, fn }) => {
  return (
    <button
      type="submit"
      className="bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-white py-3 px-6 rounded shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100"
      onClick={fn}
    >
      {label}
    </button>
  );
};

export default Button;
