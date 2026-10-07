import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useRef, useState } from "react";
import { Navbar } from "@/components/Navbar/Navbar";
import { VotePanel, phaseToVoteState } from "@/components/VotePanel/VotePanel";
import { Panel } from "@/components/Panel/Panel";
import { Spinner } from "@/components/Spinner/Spinner";
import { isLoggedOut } from "@/api/error";
import {
  type MeetingView,
  ensureTrustauthSession,
  getMeeting,
  getSession,
  watchMeeting,
} from "@/api/meeting";

export const Route = createFileRoute("/meeting")({
  component: MeetingPage,
});

/** A safety net in case an event is missed; the event stream does the real work. */
const SESSION_POLL_MS = 10_000;

function MeetingPage() {
  // null = still checking, true = in meeting, false = removed/not logged in
  const [sessionValid, setSessionValid] = useState<boolean | null>(null);
  const [meetingId, setMeetingId] = useState<string | null>(null);
  const [view, setView] = useState<MeetingView | null>(null);
  const roundVersion = useRef<number | null>(null);

  const refresh = useCallback(async () => {
    try {
      const v = await getMeeting();
      roundVersion.current = v.roundVersion;
      setView(v);
      setSessionValid(true);
    } catch (err) {
      if (isLoggedOut(err)) setSessionValid(false);
      // Other errors (network, server) keep the current state; the next event or poll retries.
    }
  }, []);

  // ── Initial load + periodic check ───────────────────────────────────────────
  useEffect(() => {
    getSession()
      .then((s) => {
        setMeetingId(s.meeting);
        // Repairs a trustauth login interrupted by a closed tab or lost connection.
        return ensureTrustauthSession();
      })
      .catch((err) => {
        if (isLoggedOut(err)) setSessionValid(false);
      });
    refresh();
    const timer = setInterval(refresh, SESSION_POLL_MS);
    return () => clearInterval(timer);
  }, [refresh]);

  // ── Live updates: refetch only when a round opens, closes or resets ─────────
  useEffect(() => {
    if (sessionValid !== true) return;
    return watchMeeting((versions) => {
      if (versions === null || versions.round !== roundVersion.current)
        refresh();
    });
  }, [sessionValid, refresh]);

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
            key={view?.round?.id ?? "vote"}
            voteState={phaseToVoteState(view?.phase ?? "idle")}
            voteName={view?.round?.name ?? null}
            round={view?.round ?? null}
            meetingId={meetingId}
          />
        </div>
      </main>
    </div>
  );
}
