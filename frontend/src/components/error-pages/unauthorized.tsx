import React from "react";
import { useNavigate } from "@tanstack/react-router";

export const Unauthorized: React.FC = () => {
  const navigate = useNavigate();

  function home() {
    navigate({ to: "/" });
  }

  return (
    <div className="min-h-screen flex flex-col justify-center items-center text-center p-4">
      <h1 className="text-4xl font-bold mb-4">Obehörig</h1>
      <p className="text-lg text-gray-600 mb-6">
        Du har inte behörighet att visa denna sida.
      </p>
      <button onClick={home}>Gå till startsidan</button>
    </div>
  );
};

export default Unauthorized;
