import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { Spinner } from "@/components/Spinner/Spinner";
import { Alert } from "@/components/Alert/Alert";
import { apiFetch, saveSessionIds } from "@/signatures/voteSession";

export const Route = createFileRoute("/login")({
  validateSearch: (search: Record<string, unknown>) => ({
    muuid: (search.muuid as string) || "",
    uuuid: (search.uuuid as string) || "",
    admin_msg: search.admin_msg as string | undefined,
    admin_sig: search.admin_sig as string | undefined,
  }),
  component: LoginPage,
});

// ─── Helpers ──────────────────────────────────────────────────────────────────

function hexToBytes(hex: string): number[] {
  const bytes: number[] = [];
  for (let i = 0; i < hex.length; i += 2) {
    bytes.push(parseInt(hex.substring(i, i + 2), 16));
  }
  return bytes;
}

// ─── Page ─────────────────────────────────────────────────────────────────────

function LoginPage() {
  const { muuid, uuuid, admin_msg, admin_sig } = Route.useSearch();
  const claimsHost = admin_msg && admin_sig;
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

      const admin_cred =
        admin_msg && admin_sig
          ? { msg: hexToBytes(admin_msg), sig: admin_sig }
          : undefined;

      let res: Response;
      try {
        res = await apiFetch("/api/login", {
          method: "POST",
          body: JSON.stringify({ uuuid, muuid, admin_cred }),
        });
      } catch {
        setError("Could not reach the server. Check your connection.");
        return;
      }

      if (res.ok) {
        saveSessionIds({ muuid, uuuid });
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
