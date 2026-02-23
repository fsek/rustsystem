import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { Navbar } from "@/components/Navbar/Navbar";
import { VotePanel, type VoteState } from "@/components/VotePanel/VotePanel";
import { apiUrl } from "@/signatures/voteSession";
import { fetchVoteProgress } from "@/api/host";
import type { BallotMetaData } from "@/signatures/signatures";

export const Route = createFileRoute("/meeting")({
  component: MeetingPage,
});

function MeetingPage() {
  const [voteState, setVoteState] = useState<VoteState>("Creation");
  const [voteName, setVoteName] = useState<string | null>(null);
  const [metadata, setMetadata] = useState<BallotMetaData | null>(null);

  // ── Initial load ────────────────────────────────────────────────────────────
  useEffect(() => {
    fetchVoteProgress()
      .then((p) => {
        setVoteName(p.voteName);
        setMetadata(p.metadata);
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
        if (raw === "Voting") {
          fetchVoteProgress()
            .then((p) => {
              setVoteName(p.voteName);
              setMetadata(p.metadata);
            })
            .catch(console.error);
        }
        if (raw === "Creation") {
          setVoteName(null);
          setMetadata(null);
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
            voteName={voteName}
            metadata={metadata}
          />
        </div>
      </main>
    </div>
  );
}
