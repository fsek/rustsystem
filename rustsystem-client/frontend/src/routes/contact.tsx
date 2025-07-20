import { createFileRoute } from '@tanstack/react-router'
import Header from '@/components/defaults/header';
import Footer from '@/components/defaults/footer';
import MainSection from '@/components/templates/main';
import FormSection from '@/components/templates/form';
import '@/colors.css';


export const Route = createFileRoute('/contact')({
  component: RouteComponent,
})

function RouteComponent() {
  function submit(data: Record<string, string>) {

    fetch("api/placeholder", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data)
    });
  }

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
      <Header />

      <MainSection title="Get in Touch" description=<p className="text-lg mb-6 opacity-80">
        We’d love to hear from you. Whether it’s feedback, questions, or bug reports, reach out!
      </p> />

      <FormSection
        fields={[
          { label: "Name", id: "name", type: "text" },
          { label: "Email", id: "email", type: "email" },
          { label: "Message", id: "message", type: "text" },
        ]}
        submit={{ label: "Send Message", data: submit }}
      />

      <Footer />
    </div>
  );
}
