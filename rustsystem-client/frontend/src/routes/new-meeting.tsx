import { createFileRoute, useNavigate } from "@tanstack/react-router"

import Header from "@/components/defaults/header"
import MainSection from "@/components/templates/main"
import FormSection from "@/components/templates/form"
import Footer from "@/components/defaults/footer"
import { CreateMeeting, type CreateMeetingRequest } from "@/api/createMeeting"

export const Route = createFileRoute('/new-meeting')({
  component: RouteComponent,
})

function RouteComponent() {
  const navigate = useNavigate();

  function submit(data: Record<string, string>) {
    CreateMeeting(data as CreateMeetingRequest).then((res_data) => {
      navigate({ to: "/meeting", search: { muid: res_data.muid, uuid: res_data.uuid } });
    });
  }

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
      <Header />
      <MainSection title="Create New Meeting" description=<p>Create a new meeting that suits your needs</p> />
      <FormSection
        fields={[
          { label: "Title", id: "title", type: "text" }
        ]}
        submit={{ label: "Create Meeting", data: submit }}
      />
      <Footer />
    </div>
  );
}
