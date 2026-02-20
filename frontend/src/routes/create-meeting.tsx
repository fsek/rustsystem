import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { Navbar } from "@/components/Navbar/Navbar";
import { Panel } from "@/components/Panel/Panel";
import { Input } from "@/components/Input/Input";
import { Button } from "@/components/Button/Button";
import { Alert } from "@/components/Alert/Alert";
import { Spinner } from "@/components/Spinner/Spinner";
import { createMeeting, saveSessionIds } from "@/signatures/voteSession";

export const Route = createFileRoute("/create-meeting")({
  component: CreateMeetingPage,
});

function CreateMeetingPage() {
  const navigate = useNavigate();
  const [title, setTitle] = useState("");
  const [hostName, setHostName] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const trimTitle = title.trim();
    const trimHost = hostName.trim();
    if (!trimTitle || !trimHost) return;

    setLoading(true);
    setError(null);
    try {
      const ids = await createMeeting(trimTitle, trimHost);
      saveSessionIds(ids);
      navigate({ to: "/admin" });
    } catch (err) {
      setError(String(err));
      setLoading(false);
    }
  }

  return (
    <div
      className="min-h-screen flex flex-col"
      style={{ backgroundColor: "var(--pageBg)" }}
    >
      <Navbar />
      <main className="flex-1 flex items-center justify-center px-6 py-10">
        <div className="w-full max-w-sm">
          <Panel title="New Meeting">
            <form onSubmit={handleSubmit} className="flex flex-col gap-5">
              <div className="flex flex-col gap-1.5">
                <label
                  htmlFor="meeting-title"
                  className="text-xs font-semibold uppercase tracking-wider"
                  style={{ color: "var(--textSecondary)" }}
                >
                  Meeting title
                </label>
                <Input
                  id="meeting-title"
                  size="m"
                  color="primary"
                  placeholder="e.g. Annual General Meeting"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  autoFocus
                  disabled={loading}
                />
              </div>

              <div className="flex flex-col gap-1.5">
                <label
                  htmlFor="host-name"
                  className="text-xs font-semibold uppercase tracking-wider"
                  style={{ color: "var(--textSecondary)" }}
                >
                  Your name
                </label>
                <Input
                  id="host-name"
                  size="m"
                  color="primary"
                  placeholder="e.g. Jane Smith"
                  value={hostName}
                  onChange={(e) => setHostName(e.target.value)}
                  disabled={loading}
                />
              </div>

              {error && (
                <Alert size="sm" color="accent">
                  {error}
                </Alert>
              )}

              <Button
                size="m"
                color="buttonPrimary"
                variant="filled"
                type="submit"
                disabled={loading || !title.trim() || !hostName.trim()}
              >
                {loading ? (
                  <span className="flex items-center gap-2">
                    <Spinner size="s" color="primary" />
                    Creating…
                  </span>
                ) : (
                  "Create Meeting"
                )}
              </Button>
            </form>
          </Panel>
        </div>
      </main>
    </div>
  );
}
