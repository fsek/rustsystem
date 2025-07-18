import React from 'react';
import { useNavigate } from '@tanstack/react-router';

const Forbidden: React.FC = () => {
  const navigate = useNavigate();
  
  function home() {
    navigate({ to: "/" });
  };

  return (
    <div className="min-h-screen flex flex-col justify-center items-center text-center p-4">
      <h1 className="text-4xl font-bold mb-4">403 - Forbidden</h1>
      <p className="text-lg text-gray-600 mb-6">
        You don’t have permission to access this page.
      </p>
      <button onClick={home}>Go to Homepage</button>
    </div>
  );
};

export default Forbidden;
