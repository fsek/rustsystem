import { createFileRoute, useNavigate } from "@tanstack/react-router";
import Header from '@/components/defaults/header';
import Footer from '@/components/defaults/footer';
import MainSection from "@/components/templates/main";
import TilingCardSection from "@/components/templates/tiling_cards";
import '@/colors.css';

export const Route = createFileRoute("/")({
	component: App,
});

function App() {
	const navigate = useNavigate();

	function createMeeting() {
		fetch("api/create-meeting", {
			method: "POST",
			credentials: "include",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({ title: "Test Meeting" })
		}).then((res) => {
			res.json().then((data) => {
				navigate({ to: "/meeting", search: { muid: data.muid } });
			});
		});
	}

	return (
		<div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
			<Header />

			<MainSection title="Rustsystem" description=<div>
				<p className="text-lg mb-6 opacity-80">Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
				<button className="bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-[var(--color-background)] py-3 px-6 rounded-full shadow-lg transform hover:-translate-y-1 transition-all duration-300" onClick={createMeeting}>
					Create Meeting
				</button>
			</div> />

			<TilingCardSection cards={[
				{ title: "Lorem Ipsum", content: "Lorem ipsum dolor sit amet, consectetur adipiscing elit." },
				{ title: "Dolor Sit", content: "Lorem ipsum dolor sit amet, consectetur adipiscing elit." },
				{ title: "Amet Consectetur", content: "Lorem ipsum dolor sit amet, consectetur adipiscing elit." },
			]} />

			<Footer />
		</div>
	);
}
