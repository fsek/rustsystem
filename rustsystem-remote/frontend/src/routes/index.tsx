import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
	component: App,
});

function App() {
	return (
		<div>
			<header>
				<p>
					Edit <code>src/routes/index.tsx</code> and save to reload
				</p>
			</header>
		</div>
	);
}
