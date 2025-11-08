import { useEffect, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { Auth, AuthStatus, type AuthMeetingRequest } from "@/api/auth";
import Unauthorized from "@/components/error-pages/unauthorized.tsx";
import HostPage from "@/components/meeting/host";
import VoterPage from "@/components/meeting/voter";
import { matchResult } from "@/result";
import type { APIError } from "@/api/error";
import ErrorHandler from "@/components/error";

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

  if (error) {
    return <ErrorHandler error={error} />;
  }

  var page = undefined;

  if (authStatus === AuthStatus.Loading) page = <div>Authenticating...</div>;
  if (authStatus === AuthStatus.VerifiedHost) page = <HostPage muid={muuid} />;
  if (authStatus === AuthStatus.VerifiedNonHost)
    page = <VoterPage muid={muuid} uuid={uuid} />;
  if (authStatus === AuthStatus.Denied)
    page = (
      <div>
        <Unauthorized />
      </div>
    );

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
      {page}
    </div>
  );
}
