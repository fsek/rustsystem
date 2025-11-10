import { useNavigate } from "@tanstack/react-router";
import type React from "react";

export const NotFound: React.FC = () => {
	const navigate = useNavigate();

	function home() {
		navigate({ to: "/" });
	}

	return (
		<div className="min-h-screen flex flex-col justify-center items-center text-center p-4">
			<h1 className="text-4xl font-bold mb-4">404 - Sidan hittades inte</h1>
			<p className="text-lg text-gray-600 mb-6">
				Sidan du letar efter finns inte.
			</p>
			<button onClick={home}>Gå till startsidan</button>
		</div>
	);
};

export default NotFound;
