import { Auth, type AuthMeetingRequest, AuthStatus } from "@/api/auth";
import {
  MeetingSpecs,
  type MeetingSpecsRequest,
  type UpdateAgendaRequest,
  updateAgenda,
  meetingSpecsWatch,
} from "@/api/common/meetingSpecs";
import type { APIError } from "@/api/error";
import ErrorHandler from "@/components/error";

import HostPage from "@/components/meeting/host";
import FloatingControls from "@/components/meeting/host-widget/floating-controls";
import { matchResult } from "@/result";
import { createFileRoute, Link } from "@tanstack/react-router";
import { useCallback, useEffect, useRef, useState } from "react";
import "@/colors.css";

type SearchParams = {
  muuid: string;
  uuuid: string;
};

export const Route = createFileRoute("/meeting_/admin")({
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
  const [isUpdatingAgenda, setIsUpdatingAgenda] = useState(false);
  const [lastAgendaUpdate, setLastAgendaUpdate] = useState<number>(0);
  const [agendaUpdateSource, setAgendaUpdateSource] = useState<
    "local" | "remote"
  >("local");
  const [showSyncStatus, setShowSyncStatus] = useState(false);
  const debounceTimerRef = useRef<number | null>(null);
  const search = Route.useSearch();
  const muuid = search.muuid;
  const uuuid = search.uuuid;

  useEffect(() => {
    Auth({ muuid } satisfies AuthMeetingRequest).then((result) => {
      matchResult(result, {
        Ok: (res) => {
          if (res.is_host) {
            setAuthStatus(AuthStatus.VerifiedHost);
          } else {
            setAuthStatus(AuthStatus.Denied);
          }
        },
        Err: (err) => {
          setError(err);
        },
      });
    });

    // Fetch meeting specs to get agenda
    MeetingSpecs({} as MeetingSpecsRequest).then((result) => {
      matchResult(result, {
        Ok: (specsData) => {
          setAgenda(specsData.agenda);
        },
        Err: (err) => {
          setError(err);
        },
      });
    });
  }, []);

  // Setup real-time agenda synchronization
  useEffect(() => {
    const agendaEventSource = meetingSpecsWatch();

    agendaEventSource.onmessage = (event) => {
      if (event.data === "NewData") {
        // Fetch the latest agenda to sync with other admins
        MeetingSpecs({} as MeetingSpecsRequest).then((result) => {
          matchResult(result, {
            Ok: (specsData) => {
              const now = Date.now();
              // Only update if this isn't from our own recent update
              if (now - lastAgendaUpdate > 1000) {
                setAgendaUpdateSource("remote");
                setAgenda(specsData.agenda);
                setShowSyncStatus(true);
                setTimeout(() => setShowSyncStatus(false), 2000);
              }
            },
            Err: (err) => {
              console.error("Failed to fetch updated agenda:", err);
            },
          });
        });
      }
    };

    agendaEventSource.onerror = (error) => {
      console.error("Agenda watch error:", error);
    };

    return () => {
      agendaEventSource.close();
    };
  }, [lastAgendaUpdate]);

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, []);

  const debouncedSaveAgenda = useCallback(async (agendaText: string) => {
    setIsUpdatingAgenda(true);
    setLastAgendaUpdate(Date.now());
    setAgendaUpdateSource("local");
    const result = await updateAgenda({
      agenda: agendaText,
    } as UpdateAgendaRequest);
    matchResult(result, {
      Ok: () => {
        // Success - agenda saved and will be broadcast to other admins
        setShowSyncStatus(true);
        setTimeout(() => setShowSyncStatus(false), 1500);
      },
      Err: (err) => {
        setError(err);
      },
    });
    setIsUpdatingAgenda(false);
  }, []);

  const handleAgendaChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newAgenda = e.target.value;
    setAgenda(newAgenda);

    // Clear existing timer
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }

    // Set new timer
    debounceTimerRef.current = setTimeout(() => {
      debouncedSaveAgenda(newAgenda);
    }, 500);
  };

  const handleAgendaKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Tab") {
      e.preventDefault();
      const textarea = e.currentTarget;
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      const value = textarea.value;

      // Insert 4 spaces at cursor position
      const newValue =
        value.substring(0, start) + "    " + value.substring(end);
      setAgenda(newValue);

      // Move cursor to after the inserted spaces
      setTimeout(() => {
        textarea.selectionStart = textarea.selectionEnd = start + 4;
      }, 0);
    }
  };

  if (error) {
    return <ErrorHandler error={error} />;
  }

  if (authStatus === AuthStatus.Loading) {
    return (
      <div className="min-h-screen bg-[var(--color-background)] flex items-center justify-center">
        <div className="text-lg text-gray-600">Autentiserar...</div>
      </div>
    );
  }

  if (authStatus === AuthStatus.Denied) {
    return (
      <div className="min-h-screen bg-[var(--color-background)] flex items-center justify-center">
        <div className="text-center">
          <h1 className="text-2xl font-semibold text-red-600 mb-4">
            Administratörsåtkomst krävs
          </h1>
          <p className="text-gray-600 mb-6">
            Du behöver administratörsbehörigheter för att komma åt denna sida.
          </p>
          <Link
            to="/meeting"
            search={{ muuid, uuuid }}
            className="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
          >
            Gå till röstningssida
          </Link>
        </div>
      </div>
    );
  }

  // Only render admin interface if user is verified as host
  if (authStatus !== AuthStatus.VerifiedHost) {
    return (
      <div className="min-h-screen bg-[var(--color-background)] flex items-center justify-center">
        <div className="text-lg text-gray-600">Laddar...</div>
      </div>
    );
  }
  // Admin view - split panes with agenda and navigation
  return (
    <div className="h-screen bg-[var(--color-background)] flex">
      {/* Left Pane - Agenda */}
      <div className="w-1/2 border-r border-gray-200 flex flex-col">
        <div className="p-6 border-b border-gray-200 bg-white">
          <div className="flex items-center justify-between mb-2">
            <h2 className="text-xl font-semibold text-[var(--color-contours)]">
              Kollaborativ agenda
            </h2>
            <Link
              to="/meeting"
              search={{ muuid, uuuid }}
              className="inline-flex items-center px-3 py-1.5 border border-gray-300 text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
            >
              🗳️ Rösta
            </Link>
          </div>
          <div className="flex items-center justify-between">
            <p className="text-sm text-gray-600">
              Ändringar synkroniseras automatiskt mellan alla administratörer.
            </p>
            {showSyncStatus && (
              <div
                className={`text-xs px-2 py-1 rounded-full ${
                  agendaUpdateSource === "remote"
                    ? "bg-blue-100 text-blue-800"
                    : "bg-green-100 text-green-800"
                }`}
              >
                {agendaUpdateSource === "remote"
                  ? "📥 Synkroniserad"
                  : "📤 Sparat"}
              </div>
            )}
          </div>
        </div>
        <div className="flex-1 p-6 bg-white relative">
          <textarea
            value={agenda}
            onChange={handleAgendaChange}
            onKeyDown={handleAgendaKeyDown}
            placeholder="Lägg till dagordningspunkter, anteckningar och diskussionspunkter här...

🔸 Välkomna
🔸 Punkt 1: ...
🔸 Punkt 2: ...
🔸 Övrigt
🔸 Avslutning"
            className="w-full h-full resize-none border-0 focus:outline-none focus:ring-0 text-gray-700 placeholder-gray-400 text-base leading-relaxed"
            style={{
              fontFamily:
                'ui-monospace, "SF Mono", Monaco, "Cascadia Code", "Roboto Mono", Consolas, "Courier New", monospace',
            }}
          />
          {isUpdatingAgenda && (
            <div className="absolute bottom-2 right-2 text-xs text-blue-600 bg-blue-50 px-3 py-2 rounded-lg shadow-sm border border-blue-200 flex items-center gap-2">
              <div className="animate-spin rounded-full h-3 w-3 border-b-2 border-blue-600"></div>
              Sparar och synkroniserar...
            </div>
          )}
        </div>
      </div>

      {/* Right Pane - Admin Controls */}
      <div className="w-1/2 flex flex-col overflow-hidden">
        <div className="p-6 border-b border-gray-200 bg-white">
          <h2 className="text-xl font-semibold text-[var(--color-contours)]">
            Administratörskontroller
          </h2>
          <p className="text-sm text-gray-600">
            Hantera omröstningar och deltagare
          </p>
        </div>
        <div className="flex-1 overflow-y-auto">
          <HostPage muid={muuid} />
        </div>
      </div>

      {/* Floating Controls */}
      <FloatingControls muid={muuid} uuuid={uuuid} setError={setError} />
    </div>
  );
}
