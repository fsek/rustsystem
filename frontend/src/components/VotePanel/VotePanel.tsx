import { useEffect, useState } from "react";
import { Panel } from "@/components/Panel/Panel";
import { Button } from "@/components/Button/Button";
import { Spinner } from "@/components/Spinner/Spinner";
import { Alert } from "@/components/Alert/Alert";
import {
  registerVoter,
  submitVote,
  type SessionIds,
} from "@/signatures/voteSession";
import type {
  RegistrationSuccessResponse,
  GeneratedToken,
} from "@/signatures/signatures";

export type VoteState = "Creation" | "Voting" | "Tally";

export interface VotePanelProps {
  voteState: VoteState;
  session: SessionIds | null;
  voteName?: string | null;
}

type VoterStatus = "idle" | "registering" | "selecting" | "submitting" | "done";

export function VotePanel({ voteState, session, voteName }: VotePanelProps) {
  const [status, setStatus] = useState<VoterStatus>("idle");
  const [token, setToken] = useState<GeneratedToken | null>(null);
  const [regResponse, setRegResponse] =
    useState<RegistrationSuccessResponse | null>(null);
  const [selected, setSelected] = useState<number[]>([]);
  const [error, setError] = useState<string | null>(null);

  // Reset voter state between rounds (when state returns to Creation).
  useEffect(() => {
    if (voteState === "Creation") {
      setStatus("idle");
      setToken(null);
      setRegResponse(null);
      setSelected([]);
      setError(null);
    }
  }, [voteState]);

  async function handleRegister() {
    if (!session) return;
    setStatus("registering");
    setError(null);
    try {
      const { token: t, regResponse: r } = await registerVoter(session);
      setToken(t);
      setRegResponse(r);
      setSelected([]);
      setStatus("selecting");
    } catch (err) {
      setError(String(err));
      setStatus("idle");
    }
  }

  async function handleSubmit(blank = false) {
    if (!token || !regResponse) return;
    setStatus("submitting");
    setError(null);
    try {
      await submitVote(token, regResponse, blank ? null : selected);
      setStatus("done");
    } catch (err) {
      setError(String(err));
      setStatus("selecting");
    }
  }

  function toggleOption(idx: number) {
    if (!regResponse) return;
    const max = regResponse.metadata.max_choices;
    setSelected((prev) => {
      if (prev.includes(idx)) return prev.filter((i) => i !== idx);
      if (max === 1) return [idx];
      if (prev.length >= max) return prev;
      return [...prev, idx];
    });
  }

  const candidates = regResponse?.metadata.candidates ?? [];
  const maxChoices = regResponse?.metadata.max_choices ?? 1;

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
            {voteName && status !== "done" && (
              <p
                className="font-semibold text-sm"
                style={{ color: "var(--textSecondary)" }}
              >
                {voteName}
              </p>
            )}

            {status === "idle" && (
              <div className="flex flex-col gap-3">
                <p
                  className="text-sm"
                  style={{ color: "var(--textSecondary)" }}
                >
                  Register a blind token to cast your anonymous vote.
                </p>
                <Button
                  size="m"
                  color="buttonPrimary"
                  variant="filled"
                  onClick={handleRegister}
                  disabled={!session}
                >
                  Register to vote
                </Button>
              </div>
            )}

            {status === "registering" && (
              <div className="flex items-center gap-3 py-2">
                <Spinner size="m" color="primary" />
                <span
                  className="text-sm"
                  style={{ color: "var(--textSecondary)" }}
                >
                  Getting ballot…
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
                        disabled={status === "submitting"}
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
                    disabled={status === "submitting" || selected.length === 0}
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
                    disabled={status === "submitting"}
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
