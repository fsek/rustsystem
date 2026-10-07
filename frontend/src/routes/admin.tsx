import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { rankItem, compareItems } from "@tanstack/match-sorter-utils";
import { Button } from "@/components/Button/Button";
import { Input } from "@/components/Input/Input";
import { Alert } from "@/components/Alert/Alert";
import { Spinner } from "@/components/Spinner/Spinner";
import { Badge } from "@/components/Badge/Badge";
import { Panel } from "@/components/Panel/Panel";
import {
  AgendaPanel,
  type AgendaHostActions,
} from "@/components/AgendaPanel/AgendaPanel";
import {
  VotePanel,
  phaseToVoteState,
  type VoteState,
} from "@/components/VotePanel/VotePanel";
import { fromBase64 } from "@/api/encoding";
import { errorMessage, isApiError, isLoggedOut } from "@/api/error";
import {
  addVoter,
  clearAgenda,
  closeMeeting,
  closeRound,
  getAgendaSource,
  getAttendance,
  getRound,
  getTallyFiles,
  goToAgendaPoint,
  listVoters,
  removeAllVoters,
  removeVoter,
  resetRound,
  setAgenda,
  startRound,
  takeAttendance,
  toTallyResult,
  type HostRoundView,
  type Invite,
  type TallyResult,
  type VoterInfo,
} from "@/api/host";
import {
  type AgendaView,
  ensureTrustauthSession,
  getMeeting,
  getSession,
  watchMeeting,
} from "@/api/meeting";
import { decryptTallyFiles } from "@/utils/tallyDecrypt";
import {
  tallyToJson,
  tallyToYaml,
  tallyToToml,
  tallyToRon,
  tallyToBson,
} from "@/utils/tallyExport";

export const Route = createFileRoute("/admin")({
  component: Admin,
});

// ─── Types ────────────────────────────────────────────────────────────────────

type NewInvite = Invite & { voterName: string };

// ─── Label helper ─────────────────────────────────────────────────────────────

function FieldLabel({ children }: { children: ReactNode }) {
  return (
    <span
      className="text-xs font-semibold uppercase tracking-wider"
      style={{ color: "var(--textSecondary)" }}
    >
      {children}
    </span>
  );
}

// ─── Add voter panel ──────────────────────────────────────────────────────────

function AddVoterPanel({
  onAdded,
  voteState,
}: {
  onAdded: (r: NewInvite) => void;
  voteState: VoteState;
}) {
  const [name, setName] = useState("");
  const [isHost, setIsHost] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleAdd() {
    const trimmed = name.trim();
    if (!trimmed) return;
    setLoading(true);
    setError(null);
    try {
      const result = await addVoter(trimmed, isHost);
      onAdded({ ...result, voterName: trimmed });
      setName("");
      setIsHost(false);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <Panel title="Add voter">
      <div className="flex flex-col gap-4">
        <div className="flex gap-2">
          <Input
            size="m"
            color="primary"
            placeholder="Name"
            disabled={voteState !== "Creation"}
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleAdd()}
            className="flex-1"
          />
          <Button
            size="m"
            color="buttonPrimary"
            variant="filled"
            onClick={handleAdd}
            disabled={loading || !name.trim() || voteState !== "Creation"}
          >
            {loading ? <Spinner size="s" color="primary" /> : "Add"}
          </Button>
        </div>

        <label
          className="flex items-center gap-2.5 cursor-pointer select-none text-sm"
          style={{ color: "var(--textSecondary)" }}
        >
          <input
            type="checkbox"
            checked={isHost}
            onChange={(e) => setIsHost(e.target.checked)}
            className="w-4 h-4"
            style={{ accentColor: "var(--primary)" }}
          />
          Grant host privileges
        </label>

        {error && (
          <Alert size="sm" color="accent">
            {error}
          </Alert>
        )}
      </div>
    </Panel>
  );
}

// ─── QR code panel ────────────────────────────────────────────────────────────

function QRPanel({
  result,
  onDismiss,
}: {
  result: NewInvite;
  onDismiss: () => void;
}) {
  const [copied, setCopied] = useState(false);

  function copy() {
    navigator.clipboard.writeText(result.inviteLink);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <Panel
      title={`Invite for ${result.voterName}`}
      actions={
        <button
          type="button"
          onClick={onDismiss}
          className="text-xs cursor-pointer transition-opacity opacity-50 hover:opacity-100"
          style={{ color: "var(--textSecondary)" }}
        >
          Dismiss
        </button>
      }
    >
      <div className="flex flex-col gap-4">
        {/* QR code: white background ensures readability regardless of theme */}
        <div className="flex justify-center">
          <div
            className="rounded-xl p-3 inline-block"
            style={{ background: "#ffffff" }}
          >
            <img
              src={result.qrSvg}
              alt={`Invite QR code for ${result.voterName}`}
              width={200}
              height={200}
            />
          </div>
        </div>

        <div className="flex gap-2 items-center">
          <code
            className="flex-1 text-xs px-3 py-2 rounded-lg truncate"
            style={{
              background: "var(--pageBg)",
              border: "1px solid var(--border)",
              color: "var(--textSecondary)",
            }}
          >
            {result.inviteLink}
          </code>
          <Button
            size="sm"
            color="buttonSecondary"
            variant="outline"
            onClick={copy}
          >
            {copied ? "Copied!" : "Copy"}
          </Button>
        </div>
      </div>
    </Panel>
  );
}

// ─── Voter list panel ─────────────────────────────────────────────────────────

function VoterListPanel({
  voters,
  loading,
  selfId,
  onRemove,
  onRemoveAll,
  onReload,
}: {
  voters: VoterInfo[];
  loading: boolean;
  selfId: string | null;
  onRemove: (id: string) => Promise<void>;
  onRemoveAll: () => Promise<void>;
  onReload: () => void;
}) {
  const [removing, setRemoving] = useState<string | null>(null);
  const [removingAll, setRemovingAll] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchPredicate, setSearchPredicate] = useState<string>("");
  const sortedVoters = useMemo(() => {
    return voters
      .map((voter) => ({
        voter,
        rank: rankItem(voter.name, searchPredicate),
      }))
      .filter((item) => item.rank.passed)
      .sort((a, b) => compareItems(a.rank, b.rank))
      .map((item) => item.voter);
  }, [searchPredicate, voters]);

  async function handleRemove(id: string) {
    setRemoving(id);
    setError(null);
    try {
      await onRemove(id);
      onReload();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setRemoving(null);
    }
  }

  async function handleRemoveAll() {
    setRemovingAll(true);
    setError(null);
    try {
      await onRemoveAll();
      onReload();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setRemovingAll(false);
    }
  }

  const nonHostCount = voters.filter((v) => !v.isHost).length;

  return (
    <Panel
      title={`Voters (${voters.length})`}
      noPad
      actions={
        nonHostCount > 0 ? (
          <Button
            size="s"
            color="buttonSecondary"
            variant="outline"
            onClick={handleRemoveAll}
            disabled={removingAll}
          >
            {removingAll ? (
              <Spinner size="s" color="secondary" />
            ) : (
              "Remove all"
            )}
          </Button>
        ) : undefined
      }
    >
      {loading ? (
        <div className="flex justify-center py-8">
          <Spinner size="m" color="primary" />
        </div>
      ) : voters.length === 0 ? (
        <p
          className="text-sm text-center py-8"
          style={{ color: "var(--textSecondary)" }}
        >
          No voters yet.
        </p>
      ) : (
        <div className="flex flex-col overflow-y-auto max-h-72">
          <div
            className="flex items-center gap-2 px-5 py-2"
            style={{ borderBottom: "1px solid var(--border)" }}
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 16 16"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
              aria-hidden="true"
              style={{ color: "var(--textSecondary)", flexShrink: 0 }}
            >
              <path
                d="M10 6.5C10 8.433 8.433 10 6.5 10C4.567 10 3 8.433 3 6.5C3 4.567 4.567 3 6.5 3C8.433 3 10 4.567 10 6.5ZM9.30884 10.0159C8.53901 10.6318 7.56251 11 6.5 11C4.01472 11 2 8.98528 2 6.5C2 4.01472 4.01472 2 6.5 2C8.98528 2 11 4.01472 11 6.5C11 7.56251 10.6318 8.53901 10.0159 9.30884L12.8536 12.1464C13.0488 12.3417 13.0488 12.6583 12.8536 12.8536C12.6583 13.0488 12.3417 13.0488 12.1464 12.8536L9.30884 10.0159Z"
                fill="currentColor"
                fillRule="evenodd"
                clipRule="evenodd"
              />
            </svg>
            <Input
              size="sm"
              color="secondary"
              placeholder="Search name"
              value={searchPredicate}
              onChange={(e) => setSearchPredicate(e.target.value)}
              className="flex-1"
            />
          </div>
          {sortedVoters.map((v, _) => (
            <div
              key={v.id}
              className="flex items-center gap-3 px-5 py-3"
              style={{
                borderTop: "1px solid var(--border)",
              }}
            >
              {/* Online dot */}
              <span
                className="w-2 h-2 rounded-full shrink-0"
                style={{
                  background: v.loggedIn
                    ? "var(--primary)"
                    : "color-mix(in srgb, var(--border) 200%, transparent)",
                }}
              />

              <span
                className="flex-1 text-sm font-medium truncate"
                style={{ color: "var(--textPrimary)" }}
              >
                {v.name}
              </span>

              {v.isHost && (
                <Badge size="s" color="primary" textColor="textPrimary">
                  host
                </Badge>
              )}

              {v.id !== selfId && (
                <button
                  type="button"
                  onClick={() => handleRemove(v.id)}
                  disabled={removing === v.id}
                  className="shrink-0 w-6 h-6 flex items-center justify-center rounded-lg text-base leading-none cursor-pointer transition-opacity opacity-30 hover:opacity-80 disabled:opacity-20"
                  style={{ color: "var(--textSecondary)" }}
                >
                  {removing === v.id ? (
                    <Spinner size="s" color="secondary" />
                  ) : (
                    "×"
                  )}
                </button>
              )}
            </div>
          ))}
        </div>
      )}
      {error && (
        <div className="px-5 py-3">
          <Alert size="sm" color="accent">
            {error}
          </Alert>
        </div>
      )}
    </Panel>
  );
}

// ─── Vote options input ───────────────────────────────────────────────────────

function VoteOptionsInput({
  options,
  onChange,
}: {
  options: string[];
  onChange: (opts: string[]) => void;
}) {
  function update(i: number, val: string) {
    const next = [...options];
    next[i] = val;
    onChange(next);
  }

  function remove(i: number) {
    if (options.length <= 2) return;
    onChange(options.filter((_, idx) => idx !== i));
  }

  return (
    <div className="flex flex-col gap-2">
      {options.map((opt, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: stable for ordered list inputs
        <div key={i} className="flex gap-2 items-center">
          <Input
            size="m"
            color="secondary"
            placeholder={`Option ${i + 1}`}
            value={opt}
            onChange={(e) => update(i, e.target.value)}
            className="flex-1"
          />
          {options.length > 2 && (
            <button
              type="button"
              onClick={() => remove(i)}
              className="w-6 h-6 flex items-center justify-center rounded-lg text-base leading-none cursor-pointer transition-opacity opacity-30 hover:opacity-80"
              style={{ color: "var(--textSecondary)" }}
            >
              ×
            </button>
          )}
        </div>
      ))}
      <button
        type="button"
        onClick={() => onChange([...options, ""])}
        className="text-sm text-left transition-opacity opacity-60 hover:opacity-100 cursor-pointer"
        style={{ color: "var(--primary)" }}
      >
        + Add option
      </button>
    </div>
  );
}

// ─── Tally download button ────────────────────────────────────────────────────

const DOWNLOAD_FORMATS = [
  { label: "JSON", ext: "json", mime: "application/json" },
  { label: "YAML", ext: "yaml", mime: "application/x-yaml" },
  { label: "TOML", ext: "toml", mime: "application/toml" },
  { label: "RON", ext: "ron", mime: "text/plain" },
  { label: "Binary JSON", ext: "bson", mime: "application/octet-stream" },
] as const;

function TallyDownloadButton({
  tally,
  voteName,
  participants,
}: {
  tally: TallyResult;
  voteName: string;
  participants: string[];
}) {
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function handleOutside(e: MouseEvent) {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleOutside);
    return () => document.removeEventListener("mousedown", handleOutside);
  }, [open]);

  function handleDownload(fmt: (typeof DOWNLOAD_FORMATS)[number]) {
    const slug = voteName.replace(/\s+/g, "_").toLowerCase() || "tally";
    const filename = `${slug}_tally.${fmt.ext}`;
    let content: string | ArrayBuffer;
    switch (fmt.ext) {
      case "json":
        content = tallyToJson(tally, participants);
        break;
      case "yaml":
        content = tallyToYaml(tally, participants);
        break;
      case "toml":
        content = tallyToToml(tally, participants);
        break;
      case "ron":
        content = tallyToRon(tally, participants);
        break;
      case "bson":
        content = tallyToBson(tally, participants);
        break;
    }
    const blob = new Blob([content], { type: fmt.mime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    setOpen(false);
  }

  return (
    <div ref={containerRef} className="relative">
      <button
        type="button"
        title="Download Results"
        onClick={() => setOpen((v) => !v)}
        className="w-9 h-9 flex items-center justify-center rounded-lg cursor-pointer transition-all hover:opacity-80"
        style={{
          background: open ? "var(--pageBg)" : "transparent",
          border: "1px solid var(--border)",
          color: "var(--textSecondary)",
        }}
      >
        <svg
          viewBox="0 0 24 24"
          width="16"
          height="16"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <polyline points="7 10 12 15 17 10" />
          <line x1="12" y1="15" x2="12" y2="3" />
        </svg>
      </button>

      {open && (
        <div
          className="absolute right-0 top-full mt-1 min-w-36 rounded-xl overflow-hidden z-50"
          style={{
            background: "var(--surface)",
            border: "1px solid var(--border)",
            boxShadow: "0 4px 16px rgba(0,0,0,0.14)",
          }}
        >
          {DOWNLOAD_FORMATS.map((fmt) => (
            <button
              key={fmt.ext}
              type="button"
              onClick={() => handleDownload(fmt)}
              className="w-full text-left text-sm px-4 py-2.5 cursor-pointer transition-colors hover:bg-[var(--pageBg)]"
              style={{ color: "var(--textPrimary)" }}
            >
              {fmt.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Tally bar ────────────────────────────────────────────────────────────────

function TallyBar({
  candidate,
  count,
  maxCount,
}: {
  candidate: string;
  count: number;
  maxCount: number;
}) {
  const pct = maxCount > 0 ? (count / maxCount) * 100 : 0;
  return (
    <div className="flex flex-col gap-1">
      <div className="flex justify-between items-baseline text-sm">
        <span style={{ color: "var(--textPrimary)" }}>{candidate}</span>
        <span
          className="font-semibold tabular-nums"
          style={{ color: "var(--textSecondary)" }}
        >
          {count}
        </span>
      </div>
      <div
        className="h-2 rounded-full overflow-hidden"
        style={{ background: "var(--border)" }}
      >
        <div
          className="h-full rounded-full transition-all duration-500"
          style={{ width: `${pct}%`, background: "var(--linearGrad)" }}
        />
      </div>
    </div>
  );
}

// ─── Host vote round panel ────────────────────────────────────────────────────

function HostVoteRoundPanel({
  voteState,
  progress,
  tallyResult,
  participants,
  onStart,
  onTally,
  onEndRound,
}: {
  voteState: VoteState;
  progress: HostRoundView | null;
  tallyResult: TallyResult | null;
  participants: string[];
  onStart: (
    name: string,
    opts: string[],
    maxChoices: number,
    shuffle: boolean,
  ) => Promise<void>;
  onTally: () => Promise<void>;
  onEndRound: () => Promise<void>;
}) {
  const [voteName, setVoteName] = useState("");
  const [options, setOptions] = useState(["", "", ""]);
  const [maxChoices, setMaxChoices] = useState(1);
  const [shuffle, setShuffle] = useState(false);
  const [starting, setStarting] = useState(false);
  const [tallying, setTallying] = useState(false);
  const [ending, setEnding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleStart() {
    const validOpts = options.filter((o) => o.trim());
    if (validOpts.length < 2) {
      setError("At least 2 non-empty options are required.");
      return;
    }
    setStarting(true);
    setError(null);
    try {
      await onStart(
        voteName.trim() || "Vote",
        validOpts,
        Math.min(maxChoices, validOpts.length),
        shuffle,
      );
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setStarting(false);
    }
  }

  async function handleTally() {
    setTallying(true);
    setError(null);
    try {
      await onTally();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setTallying(false);
    }
  }

  async function handleEnd() {
    setEnding(true);
    setError(null);
    try {
      await onEndRound();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setEnding(false);
    }
  }

  const stateBadgeColor: Record<VoteState, "primary" | "secondary" | "accent"> =
    { Creation: "secondary", Voting: "primary", Tally: "accent" };
  const stateLabel: Record<VoteState, string> = {
    Creation: "Idle",
    Voting: "Voting open",
    Tally: "Tally",
  };

  const roundName = progress?.round?.name ?? null;
  const counts = progress?.counts ?? null;
  const totalVotes = counts?.eligible ?? 0;
  const castVotes = counts?.received ?? 0;
  const progressPct = totalVotes > 0 ? (castVotes / totalVotes) * 100 : 0;
  // Trustauth signed more ballots than the server received: a vote was lost in transit.
  const lostVotes =
    counts && counts.signed !== null ? counts.signed - counts.received : 0;

  const sortedScores = tallyResult
    ? Object.entries(tallyResult.score).sort(([, a], [, b]) => b - a)
    : [];
  const maxCount = sortedScores.length > 0 ? sortedScores[0][1] : 1;

  return (
    <Panel
      title="Vote round"
      actions={
        <Badge
          size="sm"
          color={stateBadgeColor[voteState]}
          textColor="textPrimary"
        >
          {stateLabel[voteState]}
        </Badge>
      }
    >
      <div className="flex flex-col gap-5">
        {/* ── Creation ── */}
        {voteState === "Creation" && (
          <div className="flex flex-col gap-4">
            <div className="flex flex-col gap-1.5">
              <FieldLabel>Vote name</FieldLabel>
              <Input
                size="m"
                color="primary"
                placeholder="e.g. Board election"
                value={voteName}
                onChange={(e) => setVoteName(e.target.value)}
              />
            </div>

            <div className="flex flex-col gap-1.5">
              <FieldLabel>Options</FieldLabel>
              <VoteOptionsInput options={options} onChange={setOptions} />
            </div>

            <div className="flex gap-6 items-end flex-wrap">
              <div className="flex flex-col gap-1.5">
                <FieldLabel>Max choices</FieldLabel>
                <input
                  type="number"
                  min={1}
                  max={options.filter((o) => o.trim()).length || 1}
                  value={maxChoices}
                  onChange={(e) => setMaxChoices(Number(e.target.value))}
                  className="w-16 px-2 py-2 rounded-lg text-sm text-center"
                  style={{
                    background: "var(--pageBg)",
                    border: "1px solid var(--border)",
                    color: "var(--textPrimary)",
                    outline: "none",
                  }}
                />
              </div>

              <label
                className="flex items-center gap-2.5 cursor-pointer select-none text-sm pb-0.5"
                style={{ color: "var(--textSecondary)" }}
              >
                <input
                  type="checkbox"
                  checked={shuffle}
                  onChange={(e) => setShuffle(e.target.checked)}
                  className="w-4 h-4"
                  style={{ accentColor: "var(--primary)" }}
                />
                Shuffle candidates
              </label>
            </div>

            <Button
              size="m"
              color="buttonPrimary"
              variant="filled"
              onClick={handleStart}
              disabled={starting}
            >
              {starting ? (
                <span className="flex items-center gap-2">
                  <Spinner size="s" color="primary" />
                  Starting…
                </span>
              ) : (
                "Start vote round"
              )}
            </Button>
          </div>
        )}

        {/* ── Voting ── */}
        {voteState === "Voting" && (
          <div className="flex flex-col gap-5">
            {roundName && (
              <p
                className="font-semibold text-base"
                style={{ color: "var(--textPrimary)" }}
              >
                {roundName}
              </p>
            )}

            <div className="flex flex-col gap-2">
              <div
                className="flex justify-between text-sm"
                style={{ color: "var(--textSecondary)" }}
              >
                <span>Votes cast</span>
                <span className="tabular-nums font-medium">
                  {castVotes} / {totalVotes}
                </span>
              </div>
              <div
                className="h-3 rounded-full overflow-hidden"
                style={{ background: "var(--border)" }}
              >
                <div
                  className="h-full rounded-full transition-all duration-500"
                  style={{
                    width: `${progressPct}%`,
                    background: "var(--linearGrad)",
                  }}
                />
              </div>
              <div
                className="flex justify-between text-xs"
                style={{ color: "var(--textSecondary)" }}
              >
                <span>Ballots signed</span>
                <span className="tabular-nums">{counts?.signed ?? "?"}</span>
              </div>
            </div>

            <div className="flex gap-3 flex-wrap">
              <Button
                size="m"
                color="buttonPrimary"
                variant="filled"
                onClick={handleTally}
                disabled={tallying || ending}
              >
                {tallying ? (
                  <span className="flex items-center gap-2">
                    <Spinner size="s" color="primary" />
                    Tallying…
                  </span>
                ) : (
                  "Tally votes"
                )}
              </Button>
              <Button
                size="m"
                color="buttonSecondary"
                variant="outline"
                onClick={handleEnd}
                disabled={tallying || ending}
              >
                {ending ? <Spinner size="s" color="secondary" /> : "End round"}
              </Button>
            </div>
          </div>
        )}

        {/* ── Tally ── */}
        {voteState === "Tally" && (
          <div className="flex flex-col gap-5">
            {tallyResult ? (
              <div className="flex flex-col gap-3">
                {sortedScores.map(([candidate, count]) => (
                  <TallyBar
                    key={candidate}
                    candidate={candidate}
                    count={count}
                    maxCount={maxCount}
                  />
                ))}
                {tallyResult.blank > 0 && (
                  <p
                    className="text-sm pt-1"
                    style={{ color: "var(--textSecondary)" }}
                  >
                    Blank votes: {tallyResult.blank}
                  </p>
                )}
                {lostVotes > 0 && (
                  <Alert size="sm" color="accent">
                    {lostVotes === 1
                      ? "One ballot was"
                      : `${lostVotes} ballots were`}{" "}
                    signed but never arrived. A voter likely closed their page
                    mid-vote. Consider running this round again.
                  </Alert>
                )}
              </div>
            ) : (
              <div className="flex justify-center py-4">
                <Spinner size="m" color="primary" />
              </div>
            )}

            <div className="flex items-center gap-3">
              <Button
                size="m"
                color="buttonSecondary"
                variant="filled"
                onClick={handleEnd}
                disabled={ending}
              >
                {ending ? (
                  <span className="flex items-center gap-2">
                    <Spinner size="s" color="secondary" />
                    Resetting…
                  </span>
                ) : (
                  "End round"
                )}
              </Button>
              {tallyResult && (
                <TallyDownloadButton
                  tally={tallyResult}
                  voteName={roundName ?? "tally"}
                  participants={participants}
                />
              )}
            </div>
          </div>
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

// ─── Admin page ───────────────────────────────────────────────────────────────

/** A safety net in case an event is missed; the event stream does the real work. */
const SESSION_POLL_MS = 10_000;
/** Many ballots can arrive in a second; refetch at most this often. */
const REFRESH_THROTTLE_MS = 500;

function Admin() {
  const navigate = useNavigate();
  const [voters, setVoters] = useState<VoterInfo[]>([]);
  const [votersLoading, setVotersLoading] = useState(true);
  const [round, setRound] = useState<HostRoundView | null>(null);
  const [agenda, setAgendaView] = useState<AgendaView | null>(null);
  const [qrInfo, setQrInfo] = useState<NewInvite | null>(null);
  const [joinedVoterName, setJoinedVoterName] = useState<string | null>(null);
  const [confirmClose, setConfirmClose] = useState(false);
  const [closing, setClosing] = useState(false);
  const [closeError, setCloseError] = useState<string | null>(null);
  const [tallyPassword, setTallyPassword] = useState("");
  const [downloadingTallies, setDownloadingTallies] = useState(false);
  const [tallyDownloadError, setTallyDownloadError] = useState<string | null>(
    null,
  );
  const [downloadingAttendance, setDownloadingAttendance] = useState(false);
  const [attendanceError, setAttendanceError] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selfId, setSelfId] = useState<string | null>(null);
  const [meetingId, setMeetingId] = useState<string | null>(null);
  // null = still checking, true = host in a meeting, false = removed/not logged in/not a host
  const [sessionValid, setSessionValid] = useState<boolean | null>(null);

  const voteState: VoteState = phaseToVoteState(round?.phase ?? "idle");
  const tallyResult: TallyResult | null =
    round?.round && round.tally
      ? toTallyResult(round.round.candidates, round.tally)
      : null;

  // Who was logged in at the last refresh, to announce new logins.
  const loggedIn = useRef<Map<string, boolean> | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [list, r, m] = await Promise.all([
        listVoters(),
        getRound(),
        getMeeting(),
      ]);
      const before = loggedIn.current;
      if (before) {
        const joined = list.find(
          (v) => v.loggedIn && before.get(v.id) === false,
        );
        if (joined) setJoinedVoterName(joined.name);
      }
      loggedIn.current = new Map(list.map((v) => [v.id, v.loggedIn]));
      setVoters(list);
      setRound(r);
      setAgendaView(m.agenda);
      setSessionValid(true);
    } catch (err) {
      if (isLoggedOut(err) || isApiError(err, "NotHost"))
        setSessionValid(false);
      // Other errors keep the current state; the next event or poll retries.
    }
  }, []);

  // Coalesces bursts of events (e.g. many ballots at once) into one refetch.
  const refreshTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const scheduleRefresh = useCallback(() => {
    if (refreshTimer.current) return;
    refreshTimer.current = setTimeout(() => {
      refreshTimer.current = null;
      refresh();
    }, REFRESH_THROTTLE_MS);
  }, [refresh]);

  // ── Initial load ────────────────────────────────────────────────────────────
  useEffect(() => {
    async function init() {
      try {
        const session = await getSession();
        if (!session.isHost) {
          setSessionValid(false);
          return;
        }
        setSelfId(session.voter);
        setMeetingId(session.meeting);
        // Repairs a trustauth login interrupted by a closed tab or lost connection.
        ensureTrustauthSession().catch(console.warn);
        await refresh();
      } catch (err) {
        if (isLoggedOut(err)) {
          setSessionValid(false);
        } else {
          setSessionValid(true);
          setLoadError(errorMessage(err));
        }
      } finally {
        setVotersLoading(false);
      }
    }
    init();
  }, [refresh]);

  // ── Live updates ────────────────────────────────────────────────────────────
  useEffect(() => {
    if (sessionValid !== true) return;
    return watchMeeting(scheduleRefresh);
  }, [sessionValid, scheduleRefresh]);

  // ── Periodic check ──────────────────────────────────────────────────────────
  useEffect(() => {
    const timer = setInterval(refresh, SESSION_POLL_MS);
    return () => clearInterval(timer);
  }, [refresh]);

  // ── Handlers ────────────────────────────────────────────────────────────────

  async function handleStartVote(
    name: string,
    opts: string[],
    maxChoices: number,
    shuffle: boolean,
  ) {
    setRound(await startRound(name, opts, maxChoices, shuffle));
  }

  async function handleTally() {
    setRound(await closeRound());
  }

  async function handleEndRound() {
    await resetRound();
    await refresh();
  }

  async function handleCloseMeeting() {
    setClosing(true);
    setCloseError(null);
    try {
      await closeMeeting();
      navigate({ to: "/create-meeting" });
    } catch (err) {
      setCloseError(errorMessage(err));
      setClosing(false);
    }
  }

  const agendaActions: AgendaHostActions = {
    goTo: async (index) => setAgendaView(await goToAgendaPoint(index)),
    takeAttendance,
    loadSource: async () => (await getAgendaSource()).source,
    save: async (markdown) => setAgendaView(await setAgenda(markdown)),
    clear: async () => {
      await clearAgenda();
      setAgendaView(null);
    },
  };

  async function handleDownloadAttendance() {
    setDownloadingAttendance(true);
    setAttendanceError(null);
    try {
      const log = await getAttendance();
      const blob = new Blob([JSON.stringify(log, null, 2)], {
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "attendance.json";
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      setAttendanceError(errorMessage(err));
    } finally {
      setDownloadingAttendance(false);
    }
  }

  async function handleDownloadTallies() {
    if (!tallyPassword) return;
    setDownloadingTallies(true);
    setTallyDownloadError(null);
    try {
      const files = await getTallyFiles();
      // Decrypted here in the browser; the password never leaves this page.
      const decrypted = await decryptTallyFiles(
        files.map((f) => fromBase64(f.data)),
        tallyPassword,
      );
      const json = JSON.stringify(decrypted, null, 2);
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "tallies.json";
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      setTallyDownloadError(
        "Failed to decrypt. Check that you entered the correct password.",
      );
      console.error(err);
    } finally {
      setDownloadingTallies(false);
    }
  }

  function handleVoterAdded(r: NewInvite) {
    setQrInfo(r);
    setJoinedVoterName(null);
    refresh();
  }

  // ── Render ──────────────────────────────────────────────────────────────────

  if (sessionValid === null) {
    return (
      <div className="flex items-center justify-center py-32">
        <Spinner size="l" color="primary" />
      </div>
    );
  }

  if (!sessionValid) {
    return (
      <div className="max-w-md mx-auto px-6 py-10">
        <Panel title="Not an administrator">
          <p className="text-sm" style={{ color: "var(--textSecondary)" }}>
            You are not currently an administrator in a meeting. You may have
            been removed or your session may have expired. If you believe this
            is a mistake, please contact your meeting administrator.
          </p>
        </Panel>
      </div>
    );
  }

  if (loadError) {
    return (
      <div className="max-w-xl mx-auto p-8">
        <Alert size="m" color="accent">
          Failed to load admin panel: {loadError}
        </Alert>
      </div>
    );
  }

  return (
    <div
      className="max-w-7xl mx-auto px-6 py-8 flex flex-col gap-6"
      style={{ color: "var(--textPrimary)" }}
    >
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1
            className="text-3xl font-black"
            style={{ color: "var(--textPrimary)" }}
          >
            Admin
          </h1>
          <p className="text-sm mt-1" style={{ color: "var(--textSecondary)" }}>
            Manage voters and voting rounds.
          </p>
        </div>
        <Button
          size="sm"
          color="buttonSecondary"
          variant="outline"
          onClick={() => {
            setConfirmClose(true);
            setCloseError(null);
            setTallyDownloadError(null);
            setAttendanceError(null);
            setTallyPassword("");
          }}
        >
          Close meeting
        </Button>
      </div>

      {confirmClose && (
        <Panel title="Close meeting">
          <div className="flex flex-col gap-4">
            <p className="text-sm" style={{ color: "var(--textPrimary)" }}>
              Are you sure you want to close this meeting?
            </p>
            <p className="text-sm" style={{ color: "var(--textSecondary)" }}>
              All data will be permanently lost. Download all tally records
              below before closing.
            </p>

            {closeError && (
              <Alert size="sm" color="accent">
                {closeError}
              </Alert>
            )}
            <div className="flex gap-3">
              <Button
                size="m"
                color="buttonPrimary"
                variant="filled"
                onClick={handleCloseMeeting}
                disabled={closing}
              >
                {closing ? (
                  <span className="flex items-center gap-2">
                    <Spinner size="s" color="primary" />
                    Closing…
                  </span>
                ) : (
                  "Yes, close meeting"
                )}
              </Button>
              <Button
                size="m"
                color="buttonSecondary"
                variant="outline"
                onClick={() => setConfirmClose(false)}
                disabled={closing}
              >
                Cancel
              </Button>
            </div>

            <div
              className="flex flex-col gap-3 pt-3 border-t"
              style={{ borderColor: "var(--borderPrimary)" }}
            >
              <p
                className="text-xs font-semibold uppercase tracking-wider"
                style={{ color: "var(--textSecondary)" }}
              >
                Download tallies
              </p>
              <p className="text-xs" style={{ color: "var(--textSecondary)" }}>
                Enter your meeting password to decrypt and download all tally
                records as a JSON file.
              </p>
              <div className="flex gap-2">
                <Input
                  size="m"
                  color="primary"
                  type="password"
                  placeholder="Meeting password"
                  value={tallyPassword}
                  onChange={(e) => setTallyPassword(e.target.value)}
                  disabled={downloadingTallies || closing}
                />
                <Button
                  size="m"
                  color="buttonSecondary"
                  variant="outline"
                  onClick={handleDownloadTallies}
                  disabled={
                    downloadingTallies || closing || tallyPassword.length === 0
                  }
                >
                  {downloadingTallies ? (
                    <span className="flex items-center gap-2">
                      <Spinner size="s" color="secondary" />
                      Downloading…
                    </span>
                  ) : (
                    "Download"
                  )}
                </Button>
              </div>
              {tallyDownloadError && (
                <Alert size="sm" color="accent">
                  {tallyDownloadError}
                </Alert>
              )}
            </div>

            <div
              className="flex flex-col gap-3 pt-3 border-t"
              style={{ borderColor: "var(--borderPrimary)" }}
            >
              <p
                className="text-xs font-semibold uppercase tracking-wider"
                style={{ color: "var(--textSecondary)" }}
              >
                Download attendance
              </p>
              <p className="text-xs" style={{ color: "var(--textSecondary)" }}>
                Every attendance check taken in this meeting, with who was
                logged in at each, as a JSON file.
              </p>
              <div>
                <Button
                  size="m"
                  color="buttonSecondary"
                  variant="outline"
                  onClick={handleDownloadAttendance}
                  disabled={downloadingAttendance || closing}
                >
                  {downloadingAttendance ? (
                    <span className="flex items-center gap-2">
                      <Spinner size="s" color="secondary" />
                      Downloading…
                    </span>
                  ) : (
                    "Download attendance"
                  )}
                </Button>
              </div>
              {attendanceError && (
                <Alert size="sm" color="accent">
                  {attendanceError}
                </Alert>
              )}
            </div>
          </div>
        </Panel>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 items-start">
        {/* Left column: voter management */}
        <div className="flex flex-col gap-6">
          <AddVoterPanel onAdded={handleVoterAdded} voteState={voteState} />

          {joinedVoterName && (
            <Alert
              size="m"
              color="primary"
              dismissible
              onDismiss={() => setJoinedVoterName(null)}
            >
              {joinedVoterName} has logged in.
            </Alert>
          )}

          {qrInfo && (
            <QRPanel result={qrInfo} onDismiss={() => setQrInfo(null)} />
          )}

          <VoterListPanel
            voters={voters}
            loading={votersLoading}
            selfId={selfId}
            onRemove={removeVoter}
            onRemoveAll={removeAllVoters}
            onReload={refresh}
          />
        </div>

        {/* Right column: agenda, vote round + host voting */}
        <div className="flex flex-col gap-6">
          <AgendaPanel agenda={agenda} host={agendaActions} />
          <HostVoteRoundPanel
            voteState={voteState}
            progress={round}
            tallyResult={tallyResult}
            participants={voters.map((v) => v.name)}
            onStart={handleStartVote}
            onTally={handleTally}
            onEndRound={handleEndRound}
          />
          {voteState === "Voting" && (
            <VotePanel
              key={round?.round?.id ?? "vote"}
              voteState={voteState}
              voteName={round?.round?.name ?? null}
              round={round?.round ?? null}
              meetingId={meetingId}
            />
          )}
        </div>
      </div>
    </div>
  );
}
