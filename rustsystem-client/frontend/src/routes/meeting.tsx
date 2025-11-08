import { useEffect, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { Auth, AuthStatus, type AuthMeetingRequest } from "@/api/auth";
import Unauthorized from "@/components/error-pages/unauthorized.tsx";
import HostPage from "@/components/meeting/host";
import VoterPage from "@/components/meeting/voter";
import { matchResult } from "@/result";
import type { APIError } from "@/api/error";
import ErrorHandler from "@/components/error";
import FloatingControls from "@/components/meeting/host-widget/floating-controls";
import "@/colors.css";

type SearchParams = {
  muuid: string;
  uuuid: string;
};

export const Route = createFileRoute("/meeting")({
  validateSearch: (search): SearchParams => {
    return {
      muuid: (search.muuid as string) ?? "",
      uuuid: (search.uuuid as string) ?? "",
    };
  },

  component: RouteComponent,
});

function RouteComponent() {
  const [authStatus, setAuthStatus] = useState<AuthStatus>(AuthStatus.Loading);
  const [error, setError] = useState<APIError | null>(null);
  const [agenda, setAgenda] = useState<string>("");
  const search = Route.useSearch();
  const muuid = search.muuid;
  const uuid = search.uuuid;

  useEffect(() => {
    Auth({ muuid } satisfies AuthMeetingRequest).then((result) => {
      matchResult(result, {
        Ok: (res) => {
          if (res.is_host) {
            setAuthStatus(AuthStatus.VerifiedHost);
          } else {
            setAuthStatus(AuthStatus.VerifiedNonHost);
          }
        },
        Err: (err) => {
          setError(err);
        },
      });
    });
  }, []);

  const handleAgendaChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setAgenda(e.target.value);
  };

  if (error) {
    return <ErrorHandler error={error} />;
  }

  let rightPaneContent = null;

  if (authStatus === AuthStatus.Loading) {
    rightPaneContent = (
      <div className="flex items-center justify-center h-full">
        <div className="text-lg text-gray-600">Authenticating...</div>
      </div>
    );
  } else if (authStatus === AuthStatus.VerifiedHost) {
    rightPaneContent = <HostPage muid={muuid} />;
  } else if (authStatus === AuthStatus.VerifiedNonHost) {
    rightPaneContent = <VoterPage muid={muuid} uuid={uuid} />;
  } else if (authStatus === AuthStatus.Denied) {
    rightPaneContent = <Unauthorized />;
  }

  // Host view - split panes with agenda
  if (authStatus === AuthStatus.VerifiedHost) {
    return (
      <div className="h-screen bg-[var(--color-background)] flex">
        {/* Left Pane - Agenda */}
        <div className="w-1/2 border-r border-gray-200 flex flex-col">
          <div className="p-6 border-b border-gray-200 bg-white">
            <h2 className="text-xl font-semibold text-[var(--color-contours)] mb-2">
              Meeting Agenda
            </h2>
            <p className="text-sm text-gray-600">
              Use this space to track agenda items and notes
            </p>
          </div>
          <div className="flex-1 p-6 bg-white">
            <textarea
              value={agenda}
              onChange={handleAgendaChange}
              placeholder="Add agenda items, notes, and discussion points here..."
              className="w-full h-full resize-none border-0 focus:outline-none focus:ring-0 text-gray-700 placeholder-gray-400 text-base leading-relaxed"
              style={{
                fontFamily:
                  'ui-monospace, "SF Mono", Monaco, "Cascadia Code", "Roboto Mono", Consolas, "Courier New", monospace',
              }}
            />
          </div>
        </div>

        {/* Right Pane - Host Content */}
        <div className="w-1/2 flex flex-col overflow-hidden">
          <div className="flex-1 overflow-y-auto">{rightPaneContent}</div>
        </div>

        {/* Floating Controls */}
        <FloatingControls muid={muuid} setError={setError} />
      </div>
    );
  }

  // Voter/other views - normal full-width layout
  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
      {rightPaneContent}
    </div>
  );
}
