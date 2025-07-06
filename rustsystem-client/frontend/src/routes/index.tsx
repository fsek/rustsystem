import { createFileRoute, Link } from "@tanstack/react-router";

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
				<Link to="/invite">Invite!</Link>
				<Link to="/vote">Vote!</Link>
			</header>
		</div>
	);
}
