import { Auth, type AuthMeetingRequest, AuthStatus } from "@/api/auth";
import type { APIError } from "@/api/error";
import ErrorHandler from "@/components/error";
import Unauthorized from "@/components/error-pages/unauthorized.tsx";
import VoterPage from "@/components/meeting/voter";
import { matchResult } from "@/result";
import { createFileRoute, Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";
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
  const [isAdmin, setIsAdmin] = useState(false);
  const search = Route.useSearch();
  const muuid = search.muuid;
  const uuuid = search.uuuid;

  useEffect(() => {
    Auth({ muuid } satisfies AuthMeetingRequest).then((result) => {
      matchResult(result, {
        Ok: (res) => {
          setIsAdmin(res.is_host);

          // All authenticated users can vote - both regular users and admins
          if (res.is_host) {
            setAuthStatus(AuthStatus.VerifiedHost);
          } else {
            setAuthStatus(AuthStatus.VerifiedNonHost);
          }
        },
        Err: (err) => {
          console.error("Auth error:", err);
          setError(err);
        },
      });
    });
  }, []);

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
    return <Unauthorized />;
  }

  // Voter view with optional admin navigation
  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
      {/* Admin Navigation Bar */}
      {isAdmin && (
        <div className="bg-white border-b border-gray-200 shadow-sm">
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
            <div className="flex items-center justify-between h-16">
              <div className="flex items-center space-x-4">
                <div className="flex items-center">
                  <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-purple-100 text-purple-800">
                    🔐 Administratör
                  </span>
                </div>
                <div className="text-sm text-gray-600">
                  Du är inloggad som administratör
                </div>
              </div>
              <div className="flex items-center space-x-3">
                <Link
                  to="/meeting/admin"
                  search={{ muuid, uuuid }}
                  className="inline-flex items-center px-4 py-2 border border-gray-300 text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 transition-colors"
                >
                  ⚙️ Administrera möte
                </Link>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Voter Content */}
      <VoterPage muid={muuid} uuuid={uuuid} />
    </div>
  );
}
