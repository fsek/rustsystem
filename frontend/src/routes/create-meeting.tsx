import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { Navbar } from "@/components/Navbar/Navbar";
import { Panel } from "@/components/Panel/Panel";
import { Input } from "@/components/Input/Input";
import { Button } from "@/components/Button/Button";
import { Alert } from "@/components/Alert/Alert";
import { Spinner } from "@/components/Spinner/Spinner";
import { createMeeting, saveSessionIds } from "@/signatures/voteSession";
import {
  deriveEd25519PublicKeyFromPassword,
  x25519PublicKeyToPem,
} from "@/utils/cryptoGen";

const SALT_HEX = import.meta.env.SALT_HEX as string;
const ITERATIONS = import.meta.env.KEYGEN_ITERATIONS as number;

export const Route = createFileRoute("/create-meeting")({
  component: CreateMeetingPage,
});

function EyeIcon({ open }: { open: boolean }) {
  return open ? (
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
      <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  ) : (
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
      <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" />
      <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
      <line x1="1" y1="1" x2="23" y2="23" />
    </svg>
  );
}

function PasswordInput({
  id,
  placeholder,
  value,
  onChange,
  disabled,
}: {
  id: string;
  placeholder: string;
  value: string;
  onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  disabled?: boolean;
}) {
  const [visible, setVisible] = useState(false);
  return (
    <div
      className="flex items-center transition-all duration-200"
      style={{
        border: "1.5px solid var(--primary)",
        borderRadius: "0.5rem",
        backgroundColor: "var(--surface)",
      }}
    >
      <input
        id={id}
        type={visible ? "text" : "password"}
        placeholder={placeholder}
        value={value}
        onChange={onChange}
        disabled={disabled}
        className="flex-1 bg-transparent text-sm px-3 py-2 outline-none disabled:cursor-not-allowed disabled:opacity-50"
        style={{ color: "var(--textPrimary)" }}
      />
      <button
        type="button"
        aria-label={visible ? "Hide password" : "Show password"}
        onClick={() => setVisible((v) => !v)}
        disabled={disabled}
        className="px-2.5 flex items-center cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
        style={{ color: "var(--textSecondary)" }}
      >
        <EyeIcon open={visible} />
      </button>
    </div>
  );
}

function CreateMeetingPage() {
  const navigate = useNavigate();
  const [title, setTitle] = useState("");
  const [hostName, setHostName] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const trimTitle = title.trim();
    const trimHost = hostName.trim();
    if (!trimTitle || !trimHost || !password) return;

    if (password !== confirmPassword) {
      setError("Passwords do not match.");
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const publicKey = await deriveEd25519PublicKeyFromPassword({
        password,
        saltHex: SALT_HEX,
        iterations: ITERATIONS,
      });
      console.log(SALT_HEX);
      console.log(ITERATIONS);
      console.log(password);
      const ids = await createMeeting(
        trimTitle,
        trimHost,
        x25519PublicKeyToPem(publicKey),
      );
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

              <div className="flex flex-col gap-1.5">
                <label
                  htmlFor="meeting-password"
                  className="text-xs font-semibold uppercase tracking-wider"
                  style={{ color: "var(--textSecondary)" }}
                >
                  Password
                </label>
                <PasswordInput
                  id="meeting-password"
                  placeholder="Use a strong password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  disabled={loading}
                />
              </div>

              <div className="flex flex-col gap-1.5">
                <label
                  htmlFor="confirm-password"
                  className="text-xs font-semibold uppercase tracking-wider"
                  style={{ color: "var(--textSecondary)" }}
                >
                  Confirm Password
                </label>
                <PasswordInput
                  id="confirm-password"
                  placeholder="Repeat your password"
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
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
                disabled={
                  loading || !title.trim() || !hostName.trim() || !password
                }
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
