import { CreateMeeting, type CreateMeetingRequest } from "@/api/createMeeting";
import type { APIError } from "@/api/error";
import Footer from "@/components/defaults/footer";
import Header from "@/components/defaults/header";
import ErrorHandler from "@/components/error";
import { matchResult } from "@/result";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import "@/colors.css";

export const Route = createFileRoute("/")({
  component: App,
});

function App() {
  const navigate = useNavigate();
  const [error, setError] = useState<APIError | null>(null);
  const [formData, setFormData] = useState({
    host_name: "",
    title: "",
  });

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { id, value } = e.target;
    setFormData((prev) => ({ ...prev, [id]: value }));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    CreateMeeting(formData as CreateMeetingRequest).then((result) => {
      matchResult(result, {
        Ok: (res) => {
          navigate({
            to: "/meeting/admin",
            search: { muuid: res.muuid, uuuid: res.uuuid },
          });
        },
        Err: (err) => {
          setError(err);
        },
      });
    });
  };

  if (error) {
    return <ErrorHandler error={error} />;
  }

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans flex flex-col">
      <Header />

      <main className="flex-1 flex items-center justify-center px-4">
        <div className="max-w-md w-full">
          <div className="text-center mb-8">
            <h1 className="text-5xl font-bold text-[var(--color-main)] mb-4 tracking-tight">
              Rustsystem
            </h1>
            <p className="text-lg text-gray-600 leading-relaxed">
              Enkel, säker möteshantering
            </p>
          </div>

          <form
            onSubmit={handleSubmit}
            className="bg-white border border-gray-200 rounded-lg p-8 shadow-sm space-y-6"
          >
            <div>
              <label
                className="block text-sm mb-2 text-gray-700 font-medium"
                htmlFor="host_name"
              >
                Ditt namn
              </label>
              <input
                id="host_name"
                type="text"
                required
                value={formData.host_name}
                onChange={handleChange}
                className="w-full p-3 rounded border border-gray-300 focus:outline-none focus:ring-2 focus:ring-[var(--color-main)] focus:border-transparent transition-all duration-100"
                placeholder="Ange ditt namn"
              />
            </div>

            <div>
              <label
                className="block text-sm mb-2 text-gray-700 font-medium"
                htmlFor="title"
              >
                Mötestitel
              </label>
              <input
                id="title"
                type="text"
                required
                value={formData.title}
                onChange={handleChange}
                className="w-full p-3 rounded border border-gray-300 focus:outline-none focus:ring-2 focus:ring-[var(--color-main)] focus:border-transparent transition-all duration-100"
                placeholder="Ange mötestitel"
              />
            </div>

            <button
              type="submit"
              className="w-full bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-white py-3 px-6 rounded shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100 font-semibold"
            >
              Skapa möte
            </button>
          </form>
        </div>
      </main>

      <Footer />
    </div>
  );
}
