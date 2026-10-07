import { useRef, useState } from "react";
import { Panel } from "@/components/Panel/Panel";
import { Button } from "@/components/Button/Button";
import { Spinner } from "@/components/Spinner/Spinner";
import { Alert } from "@/components/Alert/Alert";
import { errorMessage } from "@/api/error";
import type { AgendaView } from "@/api/meeting";
import type { Attendance } from "@/api/host";

/** What a host can do. Without it the panel is read-only, as voters see it. */
export interface AgendaHostActions {
  goTo: (index: number) => Promise<void>;
  takeAttendance: () => Promise<Attendance>;
  /** The Markdown of the current agenda, for editing. */
  loadSource: () => Promise<string>;
  save: (markdown: string) => Promise<void>;
  clear: () => Promise<void>;
}

export interface AgendaPanelProps {
  agenda: AgendaView | null;
  host?: AgendaHostActions;
}

type Busy = "move" | "attendance" | "edit" | "save" | "clear" | null;

/**
 * The meeting agenda (`docs/PROTOCOL.md` §4.5). Voters and hosts see the same list with the
 * current point highlighted; hosts also get controls to move, edit and take attendance.
 */
export function AgendaPanel({ agenda, host }: AgendaPanelProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState<Busy>(null);
  const [error, setError] = useState<string | null>(null);
  const [recorded, setRecorded] = useState<string | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);

  if (!agenda && !host) return null;

  async function run(kind: Busy, action: () => Promise<void>) {
    setBusy(kind);
    setError(null);
    try {
      await action();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(null);
    }
  }

  const startEditing = () =>
    run("edit", async () => {
      setDraft(agenda && host ? await host.loadSource() : "");
      setRecorded(null);
      setEditing(true);
    });

  const save = () =>
    run("save", async () => {
      await host?.save(draft);
      setEditing(false);
    });

  const clear = () =>
    run("clear", async () => {
      await host?.clear();
      setEditing(false);
    });

  const goTo = (index: number) => run("move", () => host!.goTo(index));

  const takeAttendance = () =>
    run("attendance", async () => {
      const a = await host!.takeAttendance();
      const n = a.present.length;
      const where = a.point ? ` at “${a.point.title}”` : "";
      setRecorded(
        `Attendance recorded: ${n} ${n === 1 ? "member" : "members"}${where}.`,
      );
    });

  async function loadFile(file: File | undefined) {
    if (!file) return;
    try {
      setDraft(await file.text());
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  const errorAlert = error && (
    <div className="px-5 py-3">
      <Alert size="sm" color="accent">
        {error}
      </Alert>
    </div>
  );

  // ── Editor (hosts) ────────────────────────────────────────────────────────
  if (host && editing) {
    return (
      <Panel title="Edit agenda" noPad>
        <div className="flex flex-col gap-3 p-5">
          <p className="text-xs" style={{ color: "var(--textSecondary)" }}>
            Upload a Markdown file or write the agenda here. Every heading (#,
            ##, …) becomes an agenda point; the text under it is shown with that
            point.
          </p>
          <div>
            <input
              ref={fileInput}
              type="file"
              accept=".md,.markdown,.txt,text/markdown,text/plain"
              className="hidden"
              data-testid="agenda-file"
              onChange={(e) => {
                loadFile(e.target.files?.[0]);
                e.target.value = "";
              }}
            />
            <Button
              size="sm"
              color="buttonSecondary"
              variant="outline"
              type="button"
              onClick={() => fileInput.current?.click()}
              disabled={busy !== null}
            >
              Upload .md file
            </Button>
          </div>
          <textarea
            aria-label="Agenda (Markdown)"
            className="fsek-input block w-full font-mono text-xs px-3 py-2 rounded-lg transition-all duration-200"
            style={
              {
                "--input-focus-color": "var(--primary)",
                border: "1.5px solid var(--primary)",
                backgroundColor: "var(--surface)",
                color: "var(--textPrimary)",
                minHeight: "16rem",
                resize: "vertical",
              } as React.CSSProperties
            }
            placeholder={
              "# Opening\n\n# Election of chair\n## Nominations\n\n# Closing"
            }
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            disabled={busy !== null}
          />
          {error && (
            <Alert size="sm" color="accent">
              {error}
            </Alert>
          )}
          <div className="flex flex-wrap gap-2">
            <Button
              size="m"
              color="buttonPrimary"
              variant="filled"
              onClick={save}
              disabled={busy !== null || draft.trim() === ""}
            >
              {busy === "save" ? (
                <span className="flex items-center gap-2">
                  <Spinner
                    size="s"
                    color="primary"
                    className="[&_circle]:stroke-(--buttonPrimaryText)"
                  />
                  Saving…
                </span>
              ) : (
                "Save agenda"
              )}
            </Button>
            <Button
              size="m"
              color="buttonSecondary"
              variant="outline"
              onClick={() => {
                setEditing(false);
                setError(null);
              }}
              disabled={busy !== null}
            >
              Cancel
            </Button>
            {agenda && (
              <Button
                size="m"
                color="buttonSecondary"
                variant="outline"
                onClick={clear}
                disabled={busy !== null}
                className="ml-auto"
              >
                {busy === "clear" ? (
                  <Spinner size="s" color="secondary" />
                ) : (
                  "Remove agenda"
                )}
              </Button>
            )}
          </div>
        </div>
      </Panel>
    );
  }

  // ── No agenda yet (hosts only; voters see nothing) ────────────────────────
  if (!agenda) {
    return (
      <Panel title="Agenda">
        <div className="flex flex-col gap-3">
          <p className="text-sm" style={{ color: "var(--textSecondary)" }}>
            No agenda yet. Upload one as a Markdown file: every heading becomes
            a point you can step through.
          </p>
          <div>
            <Button
              size="m"
              color="buttonPrimary"
              variant="filled"
              onClick={startEditing}
            >
              Add agenda
            </Button>
          </div>
          {host && (
            <AttendanceRow
              busy={busy}
              recorded={recorded}
              onTake={takeAttendance}
              onDismiss={() => setRecorded(null)}
            />
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

  // ── The agenda ────────────────────────────────────────────────────────────
  const { points, current } = agenda;
  const minLevel = Math.min(...points.map((p) => p.level));
  const moving = busy === "move";

  return (
    <Panel
      title="Agenda"
      noPad
      actions={
        host && (
          <Button
            size="s"
            color="buttonSecondary"
            variant="outline"
            onClick={startEditing}
            disabled={busy !== null}
          >
            {busy === "edit" ? <Spinner size="s" color="secondary" /> : "Edit"}
          </Button>
        )
      }
    >
      <ol aria-label="Agenda points">
        {points.map((p, i) => {
          const isCurrent = i === current;
          const content = (
            <>
              <span
                className={`block text-sm ${isCurrent ? "font-semibold" : "font-medium"}`}
                style={{
                  color: isCurrent
                    ? "var(--textPrimary)"
                    : "var(--textSecondary)",
                }}
              >
                {p.title}
              </span>
              {isCurrent && p.body && (
                <span
                  className="block mt-1.5 text-xs whitespace-pre-wrap break-words"
                  style={{ color: "var(--textSecondary)" }}
                >
                  {p.body}
                </span>
              )}
            </>
          );
          const style: React.CSSProperties = {
            paddingLeft: `${1.25 + (p.level - minLevel) * 1}rem`,
            borderTop: i === 0 ? undefined : "1px solid var(--border)",
            borderLeft: `3px solid ${isCurrent ? "var(--primary)" : "transparent"}`,
            background: isCurrent
              ? "color-mix(in srgb, var(--primary) 10%, transparent)"
              : undefined,
          };
          return (
            <li
              key={`${i}-${p.title}`}
              aria-current={isCurrent ? "step" : undefined}
            >
              {host && !isCurrent ? (
                <button
                  type="button"
                  className="block w-full text-left pr-5 py-2.5 cursor-pointer transition-opacity hover:opacity-80 disabled:cursor-not-allowed"
                  style={style}
                  onClick={() => goTo(i)}
                  disabled={busy !== null}
                  title="Go to this point"
                >
                  {content}
                </button>
              ) : (
                <div className="pr-5 py-2.5" style={style}>
                  {content}
                </div>
              )}
            </li>
          );
        })}
      </ol>

      {host && (
        <div
          className="flex flex-col gap-3 px-5 py-3"
          style={{ borderTop: "1px solid var(--border)" }}
        >
          <div className="flex items-center gap-2">
            <Button
              size="sm"
              color="buttonSecondary"
              variant="outline"
              onClick={() => goTo(current - 1)}
              disabled={busy !== null || current === 0}
            >
              ← Previous
            </Button>
            <Button
              size="sm"
              color="buttonPrimary"
              variant="filled"
              onClick={() => goTo(current + 1)}
              disabled={busy !== null || current >= points.length - 1}
            >
              Next →
            </Button>
            <span
              className="ml-auto text-xs flex items-center gap-2"
              style={{ color: "var(--textSecondary)" }}
            >
              {moving && <Spinner size="s" color="secondary" />}
              {current + 1} / {points.length}
            </span>
          </div>
          <AttendanceRow
            busy={busy}
            recorded={recorded}
            onTake={takeAttendance}
            onDismiss={() => setRecorded(null)}
          />
        </div>
      )}
      {errorAlert}
    </Panel>
  );
}

function AttendanceRow({
  busy,
  recorded,
  onTake,
  onDismiss,
}: {
  busy: Busy;
  recorded: string | null;
  onTake: () => void;
  onDismiss: () => void;
}) {
  return (
    <div className="flex flex-col gap-2">
      <div>
        <Button
          size="sm"
          color="buttonSecondary"
          variant="outline"
          onClick={onTake}
          disabled={busy !== null}
        >
          {busy === "attendance" ? (
            <span className="flex items-center gap-2">
              <Spinner size="s" color="secondary" />
              Recording…
            </span>
          ) : (
            "Take attendance"
          )}
        </Button>
      </div>
      {recorded && (
        <Alert size="sm" color="primary" dismissible onDismiss={onDismiss}>
          {recorded}
        </Alert>
      )}
    </div>
  );
}
