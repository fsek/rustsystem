import { createFileRoute } from '@tanstack/react-router'
import { Header } from '../components/defaults/header';
import { Footer } from '../components/defaults/footer';
import MainSection from '@/components/templates/main';
import '../colors.css';


export const Route = createFileRoute('/contact')({
  component: RouteComponent,
})

function RouteComponent() {
  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
      <Header />

      <MainSection title="Get in Touch" description=<p className="text-lg mb-6 opacity-80">
        We’d love to hear from you. Whether it’s feedback, questions, or bug reports, reach out!
      </p> />

      <section className="container mx-auto px-4 mt-12 max-w-3xl">
        <form className="bg-[rgba(255,255,255,0.05)] backdrop-blur-sm border border-[var(--color-contours)] rounded-lg p-8 shadow-lg space-y-6">
          <div>
            <label className="block text-sm mb-2 opacity-80" htmlFor="name">Name</label>
            <input
              id="name"
              type="text"
              required
              className="w-full p-3 rounded-lg bg-transparent border border-[var(--color-contours)] focus:outline-none focus:ring-2 focus:ring-[var(--color-main)] transition"
            />
          </div>

          <div>
            <label className="block text-sm mb-2 opacity-80" htmlFor="email">Email</label>
            <input
              id="email"
              type="email"
              required
              className="w-full p-3 rounded-lg bg-transparent border border-[var(--color-contours)] focus:outline-none focus:ring-2 focus:ring-[var(--color-main)] transition"
            />
          </div>

          <div>
            <label className="block text-sm mb-2 opacity-80" htmlFor="message">Message</label>
            <textarea
              id="message"
              rows={5}
              required
              className="w-full p-3 rounded-lg bg-transparent border border-[var(--color-contours)] focus:outline-none focus:ring-2 focus:ring-[var(--color-main)] transition resize-none"
            ></textarea>
          </div>

          <button
            type="submit"
            className="bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-[var(--color-background)] py-3 px-6 rounded-full shadow-lg transform hover:-translate-y-1 transition-all duration-300"
          >
            Send Message
          </button>
        </form>
      </section>

      <Footer />
    </div>
  );
}
