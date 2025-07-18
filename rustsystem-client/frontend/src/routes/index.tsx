import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { Header } from '../components/defaults/header';
import { Footer } from '../components/defaults/footer';
import '../colors.css';

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

			{/* Hero Section */}
			<section className="container mx-auto px-4 mt-8">
				<div
					className="flex flex-col lg:flex-row items-center rounded-lg p-8 shadow-xl"
					style={{ background: `linear-gradient(135deg, var(--gradient-hero-start), var(--gradient-hero-end))` }}
				>
					<div className="lg:w-1/2 lg:pr-8 mb-6 lg:mb-0">
						<h2 className="text-5xl font-extrabold mb-4 bg-clip-text text-transparent"
							style={{ backgroundImage: `linear-gradient(90deg, var(--color-main), var(--color-accent2))` }}
						>
							Lorem Ipsum Dolor
						</h2>
						<p className="text-lg mb-6 opacity-80">Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
						<button className="bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-[var(--color-background)] py-3 px-6 rounded-full shadow-lg transform hover:-translate-y-1 transition-all duration-300" onClick={createMeeting}>
							Create Meeting
						</button>
					</div>
				</div>
			</section>

			{/* Features */}
			<section id="features" className="container mx-auto px-4 mt-12 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
				{["Lorem Ipsum", "Dolor Sit", "Amet Consectetur"].map((title, idx) => (
					<div
						key={idx}
						className="border border-[var(--color-contours)] rounded-lg p-6 text-center backdrop-blur-sm bg-[rgba(255,255,255,0.05)] hover:bg-[rgba(255,255,255,0.1)] transition-all duration-300"
					>
						<h3 className="text-2xl font-bold text-[var(--color-main)] mb-2">{title}</h3>
						<p className="opacity-80">Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
					</div>
				))}
			</section>

			<Footer />
		</div>
	);
}
