import { APIRequestError } from "@/api/error";
import { Alert } from "./Alert";

export function ErrorAlert({ error }: { error: unknown }) {
  if (!error) return null;

  if (error instanceof APIRequestError) {
    const { code, httpStatus, timestamp } = error.apiError.error;
    const { method, path } = error.apiError.endpoint;
    const time = new Date(timestamp).toLocaleTimeString();
    return (
      <Alert size="sm" color="accent">
        <div>{error.message}</div>
        <div
          className="text-xs mt-1 font-mono"
          style={{ opacity: 0.7 }}
        >
          {code} · HTTP {httpStatus} · {method} {path} · {time}
        </div>
      </Alert>
    );
  }

  return (
    <Alert size="sm" color="accent">
      {String(error)}
    </Alert>
  );
}
