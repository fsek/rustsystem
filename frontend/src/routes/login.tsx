import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { Spinner } from "@/components/Spinner/Spinner";
import { Alert } from "@/components/Alert/Alert";
import { errorMessage, isApiError } from "@/api/error";
import { login } from "@/api/meeting";

export const Route = createFileRoute("/login")({
  validateSearch: (search: Record<string, unknown>) => ({
    meeting: (search.meeting as string) || "",
    invite: (search.invite as string) || "",
  }),
  component: LoginPage,
});

// ─── Page ─────────────────────────────────────────────────────────────────────

function LoginPage() {
  const { meeting, invite } = Route.useSearch();
  const [error, setError] = useState<string | null>(null);

  const nav = useNavigate();

  // Guard against React StrictMode's double-invocation in development: an invite works once.
  const attempted = useRef(false);

  // biome-ignore lint/correctness/useExhaustiveDependencies: search params are URL-derived constants
  useEffect(() => {
    if (attempted.current) return;
    attempted.current = true;

    async function doLogin() {
      if (!meeting || !invite) {
        setError("Invalid login link: missing parameters.");
        return;
      }
      try {
        const res = await login(meeting, invite);
        nav({ to: res.isHost ? "/admin" : "/meeting" });
      } catch (err) {
        setError(
          isApiError(err, "InviteInvalid")
            ? "This invite link is not valid. It may already have been used, or the meeting may have ended."
            : errorMessage(err),
        );
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
