import { createFileRoute, useNavigate } from "@tanstack/react-router";

import Header from "@/components/defaults/header";
import MainSection from "@/components/templates/main";
import FormSection from "@/components/templates/form";
import Footer from "@/components/defaults/footer";
import { CreateMeeting, type CreateMeetingRequest } from "@/api/createMeeting";
import { matchResult } from "@/result";
import { useState } from "react";
import type { APIError } from "@/api/error";
import ErrorHandler from "@/components/error";

export const Route = createFileRoute("/new-meeting")({
  component: RouteComponent,
});

function RouteComponent() {
  const navigate = useNavigate();
  const [error, setError] = useState<APIError | null>(null);

  function submit(data: Record<string, string>) {
    CreateMeeting(data as CreateMeetingRequest).then((result) => {
      matchResult(result, {
        Ok: (res) => {
          navigate({
            to: "/meeting",
            search: { muuid: res.muuid, uuuid: res.uuuid },
          });
        },
        Err: (err) => {
          // This sould be considered highly unusual. There must be something wrong
          // with the server or with the connection to get here since the create-meeting
          // function itself doesn't return any error
          setError(err);
        },
      });
    });
  }

  if (error) {
    return <ErrorHandler error={error} />;
  }
  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
      <Header />
      <MainSection
        title="Create New Meeting"
        description=<p>Create a new meeting that suits your needs</p>
      />
      <FormSection
        fields={[
          { label: "Name", id: "host_name", type: "text" },
          { label: "Title", id: "title", type: "text" },
        ]}
        submit={{ label: "Create Meeting", data: submit }}
      />
      <Footer />
    </div>
  );
}
