import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/Button/Button";
import { Card } from "@/components/Card/Card";
import { Input } from "@/components/Input/Input";
import { Alert } from "@/components/Alert/Alert";
import { Spinner } from "@/components/Spinner/Spinner";
import {
  type RegistrationSuccessResponse,
  type GeneratedToken,
} from "@/signatures/signatures";
import {
  type SessionIds,
  API_BASE,
  loadVoteData,
  saveVoteData,
  clearVoteData,
  createMeeting,
  startVoteRound,
  endVoteRound,
  registerVoter,
  submitVote,
} from "@/signatures/voteSession";

export const Route = createFileRoute("/signature-dev")({
  component: SignatureDev,
});

// ─── Types ────────────────────────────────────────────────────────────────────

type Status = "idle" | "loading" | "success" | "error";

interface LogEntry {
  id: number;
  label: string;
  data: string;
}

// ─── Component ────────────────────────────────────────────────────────────────

function SignatureDev() {
  // Meeting session state
  const [session, setSession] = useState<SessionIds | null>(null);

  // Stored registration output
  const [storedToken, setStoredToken] = useState<GeneratedToken | null>(null);
  const [storedRegResponse, setStoredRegResponse] =
    useState<RegistrationSuccessResponse | null>(null);

  // True when state was hydrated from localStorage rather than fresh registration
  const [restoredFromStorage, setRestoredFromStorage] = useState(false);

  // Choice for vote submission
  const [choiceInput, setChoiceInput] = useState("");

  // Per-action status
  const [meetingStatus, setMeetingStatus] = useState<Status>("idle");
  const [startVoteStatus, setStartVoteStatus] = useState<Status>("idle");
  const [regStatus, setRegStatus] = useState<Status>("idle");
  const [voteStatus, setVoteStatus] = useState<Status>("idle");
  const [endRoundStatus, setEndRoundStatus] = useState<Status>("idle");

  // Shared request/response log
  const [log, setLog] = useState<LogEntry[]>([]);
  const nextIdRef = useRef(0);

  // ─── Restore from localStorage on mount ───────────────────────────────────

  useEffect(() => {
    const stored = loadVoteData();
    if (!stored) return;

    setSession({ muuid: stored.muuid, uuuid: stored.uuuid });
    setStoredToken({
      token: new Uint8Array(stored.token),
      blindFactor: new Uint8Array(stored.blindFactor),
      commitmentJson: stored.commitmentJson,
      context: stored.context,
    });
    setStoredRegResponse({
      signature: stored.signature,
      metadata: stored.metadata,
    });
    // Reflect the server state we assume is still valid
    setMeetingStatus("success");
    setStartVoteStatus("success");
    setRegStatus("success");
    setRestoredFromStorage(true);
  }, []);

  // ─── Helpers ──────────────────────────────────────────────────────────────

  function addLog(label: string, data: unknown) {
    const id = nextIdRef.current++;
    setLog((prev) => [
      { id, label, data: JSON.stringify(data, null, 2) },
      ...prev,
    ]);
  }

  function handleClearToken() {
    clearVoteData();
    setStoredToken(null);
    setStoredRegResponse(null);
    setRestoredFromStorage(false);
    setRegStatus("idle");
    // Session and vote-round status are server-side — leave them intact.
  }

  // ─── Setup meeting ────────────────────────────────────────────────────────

  async function handleCreateMeeting() {
    setMeetingStatus("loading");
    setSession(null);
    setStoredToken(null);
    setStoredRegResponse(null);
    setRestoredFromStorage(false);
    clearVoteData(); // new meeting → fresh slate

    try {
      addLog("POST /api/create-meeting", { title: "Dev Test Meeting", host_name: "Dev Host" });
      const ids = await createMeeting("Dev Test Meeting", "Dev Host");
      addLog("Meeting created", ids);
      setSession(ids);
      setMeetingStatus("success");
    } catch (err) {
      addLog("Error", String(err));
      setMeetingStatus("error");
    }
  }

  // ─── Start vote round ─────────────────────────────────────────────────────

  async function handleStartVote() {
    setStartVoteStatus("loading");
    setStoredToken(null);
    setStoredRegResponse(null);
    setRestoredFromStorage(false);
    clearVoteData(); // new vote round → prior registration is invalid

    const metadata = {
      candidates: ["Option A", "Option B", "Option C"],
      max_choices: 1,
      protocol_version: 1,
    };

    try {
      addLog("POST /api/host/start-vote", { name: "Dev Vote Round", shuffle: false, metadata });
      await startVoteRound("Dev Vote Round", false, metadata);
      addLog("Vote round started", "(empty)");
      setStartVoteStatus("success");
    } catch (err) {
      addLog("Error", String(err));
      setStartVoteStatus("error");
    }
  }

  // ─── Register ─────────────────────────────────────────────────────────────

  async function handleRegister() {
    if (!session) {
      addLog("Register skipped", "Create a meeting first.");
      return;
    }

    setRegStatus("loading");
    setStoredToken(null);
    setStoredRegResponse(null);

    try {
      addLog("POST /api/voter/register", "(generating blind commitment…)");
      const { token, regResponse } = await registerVoter(session);
      addLog("Registration response", regResponse);
      setStoredToken(token);
      setStoredRegResponse(regResponse);
      saveVoteData(session, token, regResponse); // persist to localStorage
      setRegStatus("success");
    } catch (err) {
      addLog("Error", String(err));
      setRegStatus("error");
    }
  }

  // ─── Submit vote ──────────────────────────────────────────────────────────

  async function handleSubmit() {
    if (!storedToken || !storedRegResponse) {
      addLog("Submit skipped", "Register first.");
      return;
    }

    setVoteStatus("loading");

    try {
      const choice =
        choiceInput.trim() === ""
          ? null
          : choiceInput.split(",").map((s) => {
            const n = parseInt(s.trim(), 10);
            if (Number.isNaN(n)) throw new Error(`Invalid index: "${s}"`);
            return n;
          });

      addLog("POST /api/voter/submit", {
        metadata: storedRegResponse.metadata,
        choice,
      });

      await submitVote(storedToken, storedRegResponse, choice);
      addLog("Vote accepted", "(empty — vote accepted)");

      // Token is spent — clear it immediately so it cannot be reused
      clearVoteData();
      setRestoredFromStorage(false);
      setVoteStatus("success");
    } catch (err) {
      addLog("Error", String(err));
      setVoteStatus("error");
    }
  }

  // ─── End vote round ───────────────────────────────────────────────────────

  async function handleEndRound() {
    setEndRoundStatus("loading");
    setStoredToken(null);
    setStoredRegResponse(null);
    setRestoredFromStorage(false);
    clearVoteData(); // ending the round invalidates any prior registration

    try {
      addLog("DELETE /api/host/end-vote-round", null);
      await endVoteRound();
      addLog("Vote round ended", "(empty)");
      setStartVoteStatus("idle");
      setEndRoundStatus("success");
    } catch (err) {
      addLog("Error", String(err));
      setEndRoundStatus("error");
    }
  }

  // ─── Render ───────────────────────────────────────────────────────────────

  const hasSession = session !== null;
  const hasVoteActive = startVoteStatus === "success";
  const hasRegistration = storedToken !== null && storedRegResponse !== null;

  return (
    <div
      className="max-w-3xl mx-auto p-8 flex flex-col gap-8"
      style={{ color: "var(--color-primary)" }}
    >
      <div>
        <h1 className="text-3xl font-bold">Blind Signature Dev</h1>
        <p className="text-sm opacity-60 mt-1">
          API base: <code>{API_BASE || "(relative)"}</code>
        </p>
      </div>

      {/* Restored-from-storage banner */}
      {restoredFromStorage && (
        <div data-testid="alert-restored">
          <Alert size="sm" color="secondary">
            Registration token recovered from browser storage. Submit your vote
            below, or click <strong>Clear token</strong> when done (e.g. on a
            shared computer).
          </Alert>
        </div>
      )}

      {/* Step 1 – create meeting */}
      <Card size="m" color="primary" title="Step 1 — Setup Meeting">
        <div className="flex flex-col gap-3 pt-2">
          <p className="text-sm opacity-80">
            Creates a meeting with a placeholder title and logs in as host. Sets
            the session cookie used by all subsequent calls.
          </p>
          <div className="flex items-center gap-4 flex-wrap">
            <Button
              size="m"
              color="primary"
              onClick={handleCreateMeeting}
              disabled={meetingStatus === "loading"}
              data-testid="btn-create-meeting"
            >
              {meetingStatus === "loading" ? (
                <span className="flex items-center gap-2">
                  <Spinner size="s" color="primary" />
                  Creating…
                </span>
              ) : (
                "Create Meeting"
              )}
            </Button>
            {meetingStatus === "success" && session && (
              <Alert size="sm" color="primary">
                Meeting ready — muuid: {session.muuid.slice(0, 8)}…
              </Alert>
            )}
            {meetingStatus === "error" && (
              <Alert size="sm" color="accent">
                Failed — see log.
              </Alert>
            )}
          </div>
        </div>
      </Card>

      {/* Step 2 – start vote */}
      <Card size="m" color="secondary" title="Step 2 — Start Vote Round">
        <div className="flex flex-col gap-3 pt-2">
          <p className="text-sm opacity-80">
            Starts a vote round with candidates <em>Option A / B / C</em>, max 1
            choice.
          </p>
          <div className="flex items-center gap-4 flex-wrap">
            <Button
              size="m"
              color="secondary"
              onClick={handleStartVote}
              disabled={startVoteStatus === "loading" || !hasSession}
              data-testid="btn-start-vote"
            >
              {startVoteStatus === "loading" ? (
                <span className="flex items-center gap-2">
                  <Spinner size="s" color="secondary" />
                  Starting…
                </span>
              ) : (
                "Start Vote"
              )}
            </Button>
            <Button
              size="m"
              color="accent"
              onClick={handleEndRound}
              disabled={endRoundStatus === "loading" || !hasSession}
              data-testid="btn-end-round"
            >
              {endRoundStatus === "loading" ? (
                <span className="flex items-center gap-2">
                  <Spinner size="s" color="accent" />
                  Ending…
                </span>
              ) : (
                "End Vote Round"
              )}
            </Button>
            {startVoteStatus === "success" && (
              <Alert size="sm" color="primary">
                Vote round active.
              </Alert>
            )}
            {endRoundStatus === "success" && (
              <Alert size="sm" color="primary">
                Round ended.
              </Alert>
            )}
            {(startVoteStatus === "error" || endRoundStatus === "error") && (
              <Alert size="sm" color="accent">
                Failed — see log.
              </Alert>
            )}
          </div>
        </div>
      </Card>

      {/* Step 3 – register */}
      <Card
        size="m"
        color="primary"
        title="Step 3 — Register (Get Blind Signature)"
      >
        <div className="flex flex-col gap-3 pt-2">
          <p className="text-sm opacity-80">
            Generates a 256-byte random token, computes a BBS+ commitment, and
            posts <code>POST /api/voter/register</code>. The token and
            blind_factor are saved to <strong>browser localStorage</strong> so a
            page refresh or browser restart does not lose them. On a shared or
            public computer, use <strong>Clear token</strong> after voting.
          </p>
          <div className="flex items-center gap-4 flex-wrap">
            <Button
              size="m"
              color="primary"
              onClick={handleRegister}
              disabled={
                regStatus === "loading" || !hasSession || !hasVoteActive
              }
              data-testid="btn-register"
            >
              {regStatus === "loading" ? (
                <span className="flex items-center gap-2">
                  <Spinner size="s" color="primary" />
                  Registering…
                </span>
              ) : (
                "Register"
              )}
            </Button>
            {hasRegistration && (
              <Button size="m" color="accent" onClick={handleClearToken} data-testid="btn-clear-token">
                Clear token
              </Button>
            )}
            {!hasVoteActive && hasSession && (
              <span className="text-sm opacity-50">
                Start a vote round first.
              </span>
            )}
            {regStatus === "success" && (
              <Alert size="sm" color="primary">
                Blind signature received. Token saved to browser storage.
              </Alert>
            )}
            {regStatus === "error" && (
              <Alert size="sm" color="accent">
                Failed — see log.
              </Alert>
            )}
          </div>
          {storedRegResponse && (
            <p className="text-xs opacity-60 mt-1">
              Metadata: {JSON.stringify(storedRegResponse.metadata)}
            </p>
          )}
        </div>
      </Card>

      {/* Step 4 – submit vote */}
      <Card size="m" color="secondary" title="Step 4 — Submit Vote">
        <div className="flex flex-col gap-3 pt-2">
          <p className="text-sm opacity-80">
            Builds a padded ballot with the stored token + blind_factor +
            signature and posts <code>POST /api/voter/submit</code>. The token
            is cleared from storage automatically after a successful submission.
          </p>
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium">
              Candidate indices (comma-separated; empty = blank vote)
            </label>
            <Input
              size="m"
              color="secondary"
              placeholder="e.g. 0  or  0,1  or leave empty"
              value={choiceInput}
              onChange={(e) => setChoiceInput(e.target.value)}
              data-testid="input-choice"
            />
          </div>
          <div className="flex items-center gap-4 flex-wrap">
            <Button
              size="m"
              color="secondary"
              onClick={handleSubmit}
              disabled={voteStatus === "loading" || !hasRegistration}
              data-testid="btn-submit"
            >
              {voteStatus === "loading" ? (
                <span className="flex items-center gap-2">
                  <Spinner size="s" color="secondary" />
                  Submitting…
                </span>
              ) : (
                "Submit Vote"
              )}
            </Button>
            {!hasRegistration && (
              <span className="text-sm opacity-50">Register first.</span>
            )}
            {voteStatus === "success" && (
              <Alert size="sm" color="primary">
                Vote accepted! Token cleared from storage.
              </Alert>
            )}
            {voteStatus === "error" && (
              <Alert size="sm" color="accent">
                Failed — see log.
              </Alert>
            )}
          </div>
        </div>
      </Card>

      {/* Request/response log */}
      {log.length > 0 && (
        <Card size="m" color="primary" title="Request / Response Log">
          <div className="flex flex-col gap-4 pt-2">
            {log.map((entry) => (
              <div key={entry.id} className="flex flex-col gap-1">
                <span className="text-xs font-semibold opacity-70">
                  {entry.label}
                </span>
                <pre
                  className="text-xs p-3 rounded overflow-x-auto"
                  style={{ background: "var(--color-surface)" }}
                >
                  {entry.data}
                </pre>
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}
