import { createFileRoute } from '@tanstack/react-router'
import Header from '@/components/defaults/header';
import Footer from '@/components/defaults/footer';
import MainSection from '@/components/templates/main';
import TilingCardSection from '@/components/templates/tiling_cards';
import '@/colors.css';

export const Route = createFileRoute('/about')({
  component: RouteComponent,
})

function RouteComponent() {
  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
      <Header />

      <MainSection title="About" description=<p className="text-lg mb-6 opacity-80">
        Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.
      </p> />

      <TilingCardSection cards={[
        { title: "Our Mission", content: "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua." },
        { title: "Our Vision", content: "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua." },
        { title: "Our Values", content: "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua." },
        { title: "Our Team", content: "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua." },
        { title: "Our Promise", content: "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua." },
      ]} />

      <section className="container mx-auto px-4 mt-16">
        <h3 className="text-3xl font-bold text-[var(--color-main)] mb-6 text-center">Gallery</h3>
        <div className="relative overflow-hidden">
          <div className="flex gap-6 overflow-x-auto scrollbar-hide snap-x snap-mandatory pb-4 px-1">
            {[1, 2, 3, 4, 5].map((item) => (
              <div
                key={item}
                className="min-w-[300px] h-[200px] snap-start shrink-0 rounded-lg overflow-hidden shadow-lg border border-[var(--color-contours)] bg-[rgba(255,255,255,0.05)] hover:bg-[rgba(255,255,255,0.1)] transition-all duration-300"
              >
                <div className="w-full h-full flex items-center justify-center text-xl opacity-70">
                  Lorem Image {item}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="container mx-auto px-4 mt-20">
        <h3 className="text-3xl font-bold text-[var(--color-main)] mb-6 text-center">What People Say</h3>
        <div className="flex gap-6 overflow-x-auto scrollbar-hide snap-x snap-mandatory pb-4 px-1">
          {[1, 2, 3].map((item) => (
            <div
              key={item}
              className="min-w-[350px] max-w-[400px] snap-start shrink-0 rounded-lg overflow-hidden shadow-lg border border-[var(--color-contours)] bg-[rgba(255,255,255,0.05)] hover:bg-[rgba(255,255,255,0.1)] transition-all duration-300 p-6"
            >
              <p className="opacity-80 mb-4">"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor."</p>
              <div className="text-sm text-[var(--color-main)] font-bold">— Person {item}</div>
            </div>
          ))}
        </div>
      </section>

      <Footer />
    </div>
  );
}
