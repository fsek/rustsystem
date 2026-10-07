import { useEffect, useRef, useState } from "react";
import { Panel } from "@/components/Panel/Panel";
import { Button } from "@/components/Button/Button";
import { Spinner } from "@/components/Spinner/Spinner";
import { Alert } from "@/components/Alert/Alert";
import { errorMessage, isApiError } from "@/api/error";
import type { Phase, RoundView } from "@/api/meeting";
import { signBallot, submitBallot, voteStatus } from "@/voting/ballot";

export type VoteState = "Creation" | "Voting" | "Tally";

export function phaseToVoteState(phase: Phase): VoteState {
  if (phase === "voting") return "Voting";
  if (phase === "tallied") return "Tally";
  return "Creation";
}

export interface VotePanelProps {
  voteState: VoteState;
  voteName?: string | null;
  /** The open round, from `GET /api/meeting`. */
  round?: RoundView | null;
  meetingId: string | null;
}

type VoterStatus =
  | "checking" // asking trustauth whether this voter has voted
  | "selecting" // choosing
  | "submitting" // signing and submitting
  | "done" // submitted just now
  | "voted"; // had already voted (e.g. after a refresh)

export function VotePanel({
  voteState,
  voteName,
  round,
  meetingId,
}: VotePanelProps) {
  const [status, setStatus] = useState<VoterStatus>("checking");
  const [selected, setSelected] = useState<number[]>([]);
  const [error, setError] = useState<string | null>(null);

  // A ballot trustauth has signed but the server hasn't received yet. Kept only in memory, so a
  // failed submission can be retried without asking trustauth again.
  const signed = useRef<{ prepared: Uint8Array; sig: Uint8Array } | null>(null);
  const checkedRound = useRef<string | null>(null);

  // On entering a round (or loading the page mid-round), ask trustauth whether we've voted.
  useEffect(() => {
    if (voteState !== "Voting" || !round) return;
    if (checkedRound.current === round.id) return;
    checkedRound.current = round.id;
    signed.current = null;
    setSelected([]);
    setError(null);
    setStatus("checking");

    voteStatus()
      .then((s) =>
        setStatus(s.round === round.id && s.signed ? "voted" : "selecting"),
      )
      .catch((err) => {
        setError(errorMessage(err));
        setStatus("selecting");
      });
  }, [voteState, round]);

  // Reset when the round ends.
  useEffect(() => {
    if (voteState !== "Voting") {
      checkedRound.current = null;
      signed.current = null;
      setStatus("checking");
      setSelected([]);
      setError(null);
    }
  }, [voteState]);

  // Closing the page between signing and submitting would lose the vote: ask first.
  useEffect(() => {
    if (status !== "submitting" && !signed.current) return;
    const warn = (e: BeforeUnloadEvent) => e.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [status]);

  async function handleSubmit(blank = false) {
    if (!round?.publicKey || !meetingId) return;
    setStatus("submitting");
    setError(null);
    try {
      if (!signed.current) {
        signed.current = await signBallot(
          {
            id: round.id,
            candidates: round.candidates,
            maxChoices: round.maxChoices,
            publicKey: round.publicKey,
          },
          blank ? null : selected,
        );
      }
      await submitBallot(meetingId, signed.current);
      signed.current = null;
      setStatus("done");
    } catch (err) {
      if (isApiError(err, "AlreadySigned")) {
        setStatus("voted");
        return;
      }
      setError(
        signed.current
          ? `Your vote could not be delivered yet: ${errorMessage(err)} Keep this page open and press Submit again.`
          : errorMessage(err),
      );
      setStatus("selecting");
    }
  }

  function toggleOption(idx: number) {
    if (!round || signed.current) return;
    const max = round.maxChoices;
    setSelected((prev) => {
      if (prev.includes(idx)) return prev.filter((i) => i !== idx);
      if (max === 1) return [idx];
      if (prev.length >= max) return prev;
      return [...prev, idx];
    });
  }

  const candidates = round?.candidates ?? [];
  const maxChoices = round?.maxChoices ?? 1;
  const locked = status === "submitting" || signed.current !== null;

  return (
    <Panel title="Your Vote">
      <div className="flex flex-col gap-4">
        {/* ── Creation: waiting for voting to start ── */}
        {voteState === "Creation" && (
          <div
            className="flex flex-col items-center gap-3 py-6 text-center"
            style={{ color: "var(--textSecondary)" }}
          >
            <Spinner size="m" color="secondary" />
            <p className="text-sm">Waiting for voting to start…</p>
          </div>
        )}

        {/* ── Tally: voting is over ── */}
        {voteState === "Tally" && (
          <div className="flex flex-col items-center gap-2 py-6 text-center">
            <p
              className="text-base font-semibold"
              style={{ color: "var(--textPrimary)" }}
            >
              The voting is now over.
            </p>
            {voteName && (
              <p className="text-sm" style={{ color: "var(--textSecondary)" }}>
                {voteName}
              </p>
            )}
          </div>
        )}

        {/* ── Voting ── */}
        {voteState === "Voting" && (
          <>
            {voteName && status !== "done" && status !== "voted" && (
              <p
                className="font-semibold text-sm"
                style={{ color: "var(--textSecondary)" }}
              >
                {voteName}
              </p>
            )}

            {status === "checking" && (
              <div className="flex items-center gap-3 py-2">
                <Spinner size="m" color="primary" />
                <span
                  className="text-sm"
                  style={{ color: "var(--textSecondary)" }}
                >
                  Checking vote status…
                </span>
              </div>
            )}

            {(status === "selecting" || status === "submitting") && (
              <div className="flex flex-col gap-4">
                <p
                  className="text-sm"
                  style={{ color: "var(--textSecondary)" }}
                >
                  {maxChoices === 1
                    ? "Select one option."
                    : `Select up to ${maxChoices} options.`}
                </p>

                <div className="flex flex-col gap-2">
                  {candidates.map((candidate, idx) => {
                    const isSelected = selected.includes(idx);
                    return (
                      <button
                        // biome-ignore lint/suspicious/noArrayIndexKey: stable ordered list
                        key={idx}
                        type="button"
                        onClick={() => toggleOption(idx)}
                        disabled={locked}
                        className="flex items-center gap-3 px-4 py-3 rounded-xl text-left w-full cursor-pointer transition-all"
                        style={{
                          background: isSelected
                            ? "color-mix(in srgb, var(--primary) 12%, var(--surface))"
                            : "var(--pageBg)",
                          border: `1px solid ${isSelected ? "var(--primary)" : "var(--border)"}`,
                          color: "var(--textPrimary)",
                        }}
                      >
                        <span
                          className="w-4 h-4 rounded-full shrink-0 border-2 transition-all"
                          style={{
                            borderColor: isSelected
                              ? "var(--primary)"
                              : "var(--border)",
                            background: isSelected
                              ? "var(--primary)"
                              : "transparent",
                          }}
                        />
                        <span className="text-sm font-medium">{candidate}</span>
                      </button>
                    );
                  })}
                </div>

                <div className="flex gap-3 flex-wrap">
                  <Button
                    size="m"
                    color="buttonPrimary"
                    variant="filled"
                    onClick={() => handleSubmit(false)}
                    disabled={
                      status === "submitting" ||
                      (selected.length === 0 && !signed.current)
                    }
                  >
                    {status === "submitting" ? (
                      <span className="flex items-center gap-2">
                        <Spinner size="s" color="primary" />
                        Submitting…
                      </span>
                    ) : (
                      "Submit vote"
                    )}
                  </Button>
                  <Button
                    size="m"
                    color="buttonSecondary"
                    variant="outline"
                    onClick={() => handleSubmit(true)}
                    disabled={locked}
                  >
                    Blank vote
                  </Button>
                </div>
              </div>
            )}

            {status === "done" && (
              <Alert size="m" color="primary">
                Your vote has been submitted anonymously.
              </Alert>
            )}

            {status === "voted" && (
              <Alert size="m" color="primary">
                You have already voted in this round.
              </Alert>
            )}
          </>
        )}

        {error && (
          <Alert size="sm" color="accent">
            {error}
          </Alert>
        )}
      </div>
    </Panel>
  );
}
