import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { Spinner } from "@/components/Spinner/Spinner";
import { Alert } from "@/components/Alert/Alert";
import { apiFetch, trustAuthLogin } from "@/signatures/voteSession";

export const Route = createFileRoute("/login")({
  validateSearch: (search: Record<string, unknown>) => ({
    muuid: (search.muuid as string) || "",
    uuuid: (search.uuuid as string) || "",
    admin_token: search.admin_token as string | undefined,
  }),
  component: LoginPage,
});

// ─── Page ─────────────────────────────────────────────────────────────────────

function LoginPage() {
  const { muuid, uuuid, admin_token } = Route.useSearch();
  const claimsHost = !!admin_token;
  const [error, setError] = useState<string | null>(null);

  const nav = useNavigate();

  // Guard against React StrictMode's double-invocation in development.
  const attempted = useRef(false);

  // biome-ignore lint/correctness/useExhaustiveDependencies: search params are URL-derived constants
  useEffect(() => {
    if (attempted.current) return;
    attempted.current = true;

    async function doLogin() {
      if (!muuid || !uuuid) {
        setError("Invalid login link — missing parameters.");
        return;
      }

      let res: Response;
      try {
        res = await apiFetch("/api/login", {
          method: "POST",
          body: JSON.stringify({ uuuid, muuid, admin_token }),
        });
      } catch {
        setError("Could not reach the server. Check your connection.");
        return;
      }

      if (res.ok) {
        try {
          await trustAuthLogin(uuuid, muuid);
        } catch {
          setError("Could not reach the trustauth server. Check your connection.");
          return;
        }

        if (claimsHost) {
          nav({ to: "/admin" });
        } else {
          nav({ to: "/meeting" });
        }
        return;
      }

      if (res.status === 409) {
        setError("This invite link has already been used.");
      } else if (res.status === 404) {
        setError("Meeting or voter not found. The meeting may have ended.");
      } else {
        setError(`Login failed (HTTP ${res.status}).`);
      }
    }

    doLogin();
  }, []);

  // ── Error ────────────────────────────────────────────────────────────────────

  if (error) {
    return (
      <div
        className="min-h-screen flex flex-col items-center justify-center gap-4 px-6 text-center"
        style={{ backgroundColor: "var(--pageBg)" }}
      >
        <div className="max-w-sm w-full">
          <Alert size="m" color="accent">
            {error}
          </Alert>
        </div>
      </div>
    );
  }

  // ── Loading ──────────────────────────────────────────────────────────────────

  return (
    <div
      className="min-h-screen flex flex-col items-center justify-center gap-4"
      style={{ backgroundColor: "var(--pageBg)" }}
    >
      <Spinner size="l" color="primary" />
      <p className="text-sm" style={{ color: "var(--textSecondary)" }}>
        Signing in…
      </p>
    </div>
  );
}
