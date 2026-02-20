import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { Navbar } from "@/components/Navbar/Navbar";
import { VotePanel, type VoteState } from "@/components/VotePanel/VotePanel";
import {
  apiUrl,
  loadSessionIds,
  type SessionIds,
} from "@/signatures/voteSession";
import { fetchVoteProgress } from "@/api/host";

export const Route = createFileRoute("/meeting")({
  component: MeetingPage,
});

function MeetingPage() {
  const [voteState, setVoteState] = useState<VoteState>("Creation");
  const [voteName, setVoteName] = useState<string | null>(null);
  const [session, setSession] = useState<SessionIds | null>(null);

  // ── Initial load ────────────────────────────────────────────────────────────
  useEffect(() => {
    setSession(loadSessionIds());

    fetchVoteProgress()
      .then((p) => {
        setVoteName(p.voteName);
        if (p.isTally) setVoteState("Tally");
        else if (p.isActive) setVoteState("Voting");
        else setVoteState("Creation");
      })
      .catch(console.error);
  }, []);

  // ── SSE: vote state ─────────────────────────────────────────────────────────
  useEffect(() => {
    const es = new EventSource(apiUrl("/api/common/vote-state-watch"), {
      withCredentials: true,
    });
    es.onmessage = (e) => {
      const raw = (e.data as string).replace(/^"|"$/g, "");
      if (raw === "Creation" || raw === "Voting" || raw === "Tally") {
        setVoteState(raw);
        // Refresh vote name when a new round starts.
        if (raw === "Voting") {
          fetchVoteProgress()
            .then((p) => setVoteName(p.voteName))
            .catch(console.error);
        }
        if (raw === "Creation") {
          setVoteName(null);
        }
      }
    };
    es.onerror = () => console.warn("vote-state-watch SSE disconnected");
    return () => es.close();
  }, []);

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
            session={session}
            voteName={voteName}
          />
        </div>
      </main>
    </div>
  );
}
