import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { Navbar } from "@/components/Navbar/Navbar";
import { VotePanel, type VoteState } from "@/components/VotePanel/VotePanel";
import { Panel } from "@/components/Panel/Panel";
import { Spinner } from "@/components/Spinner/Spinner";
import { apiFetch, apiUrl } from "@/signatures/voteSession";
import type { BallotMetaData } from "@/signatures/signatures";
import type { VoteProgress } from "@/api/host";

export const Route = createFileRoute("/meeting")({
  component: MeetingPage,
});

const SESSION_POLL_MS = 10_000;

function MeetingPage() {
  // null = still checking, true = in meeting, false = removed/not logged in
  const [sessionValid, setSessionValid] = useState<boolean | null>(null);
  const [voteState, setVoteState] = useState<VoteState>("Creation");
  const [voteName, setVoteName] = useState<string | null>(null);
  const [metadata, setMetadata] = useState<BallotMetaData | null>(null);

  // Fetch vote progress. Returns false when the session is no longer valid.
  const refreshProgress = useCallback(async (): Promise<boolean> => {
    let res: Response;
    try {
      res = await apiFetch("/api/common/vote-progress");
    } catch {
      return true; // network error — don't invalidate session
    }
    if (res.status === 401) {
      setSessionValid(false);
      return false;
    }
    if (!res.ok) return true; // other server error — keep current session state
    const p: VoteProgress = await res.json();
    setSessionValid(true);
    setVoteName(p.voteName);
    setMetadata(p.metadata);
    if (p.isTally) setVoteState("Tally");
    else if (p.isActive) setVoteState("Voting");
    else setVoteState("Creation");
    return true;
  }, []);

  // ── Initial load + periodic session check ───────────────────────────────────
  useEffect(() => {
    refreshProgress();
    const timer = setInterval(refreshProgress, SESSION_POLL_MS);
    return () => clearInterval(timer);
  }, [refreshProgress]);

  // ── SSE: vote state (only while session is valid) ────────────────────────────
  useEffect(() => {
    if (sessionValid !== true) return;

    const es = new EventSource(apiUrl("/api/common/vote-state-watch"), {
      withCredentials: true,
    });
    es.onmessage = (e) => {
      const raw = (e.data as string).replace(/^"|"$/g, "");
      if (raw === "Creation" || raw === "Voting" || raw === "Tally") {
        setVoteState(raw);
        if (raw === "Voting") {
          refreshProgress().catch(console.error);
        }
        if (raw === "Creation") {
          setVoteName(null);
          setMetadata(null);
        }
      }
    };
    es.onerror = () => console.warn("vote-state-watch SSE disconnected");
    return () => es.close();
  }, [sessionValid, refreshProgress]);

  // ── Render ──────────────────────────────────────────────────────────────────

  if (sessionValid === null) {
    return (
      <div
        className="min-h-screen flex items-center justify-center"
        style={{ backgroundColor: "var(--pageBg)" }}
      >
        <Spinner size="l" color="primary" />
      </div>
    );
  }

  if (!sessionValid) {
    return (
      <div
        className="min-h-screen flex flex-col"
        style={{ backgroundColor: "var(--pageBg)" }}
      >
        <Navbar />
        <main className="flex-1 flex items-start justify-center px-6 py-10">
          <div className="w-full max-w-md">
            <Panel title="Not in meeting">
              <p className="text-sm" style={{ color: "var(--textSecondary)" }}>
                You are not currently in a meeting. You may have been removed or
                your session may have expired. If you believe this is a mistake,
                please contact your meeting administrator.
              </p>
            </Panel>
          </div>
        </main>
      </div>
    );
  }

  return (
    <div
      className="min-h-screen flex flex-col"
      style={{ backgroundColor: "var(--pageBg)" }}
    >
      <Navbar />
      <main className="flex-1 flex items-start justify-center px-6 py-10">
        <div className="w-full max-w-md">
          <VotePanel
            key={voteName ?? "vote"}
            voteState={voteState}
            voteName={voteName}
            metadata={metadata}
          />
        </div>
      </main>
    </div>
  );
}
