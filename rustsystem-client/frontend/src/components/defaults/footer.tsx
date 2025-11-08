import React from "react";
import "@/colors.css";

const Footer: React.FC = () => {
  return (
    <footer className="py-4 text-center text-sm text-gray-500">
      &copy; {new Date().getFullYear()} F-sektionen at LTH
    </footer>
  );
};

export default Footer;
