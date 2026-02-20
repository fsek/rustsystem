import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";

export const Route = createFileRoute("/dev-testing")({
  component: DevTesting,
});

type Method = "GET" | "POST" | "PUT" | "DELETE";

interface RequestEntry {
  id: number;
  method: Method;
  path: string;
  body: string;
  status: number | null;
  response: string | null;
  pending: boolean;
}

function DevTesting() {
  const [method, setMethod] = useState<Method>("GET");
  const [path, setPath] = useState("/api/");
  const [body, setBody] = useState("");
  const [history, setHistory] = useState<RequestEntry[]>([]);
  const [nextId, setNextId] = useState(0);

  async function sendRequest() {
    const id = nextId;
    setNextId((n) => n + 1);

    const entry: RequestEntry = {
      id,
      method,
      path,
      body,
      status: null,
      response: null,
      pending: true,
    };
    setHistory((h) => [entry, ...h]);

    try {
      const init: RequestInit = {
        method,
        credentials: "include",
        headers: { "Content-Type": "application/json" },
      };
      if (method !== "GET" && body.trim()) {
        init.body = body;
      }

      const res = await fetch(path, init);
      const text = await res.text();

      let formatted: string;
      try {
        formatted = JSON.stringify(JSON.parse(text), null, 2);
      } catch {
        formatted = text;
      }

      setHistory((h) =>
        h.map((e) =>
          e.id === id
            ? { ...e, status: res.status, response: formatted, pending: false }
            : e,
        ),
      );
    } catch (err) {
      setHistory((h) =>
        h.map((e) =>
          e.id === id
            ? {
              ...e,
              status: 0,
              response: String(err),
              pending: false,
            }
            : e,
        ),
      );
    }
  }

  return (
    <div
      style={{ fontFamily: "monospace", padding: "1rem", maxWidth: "60rem" }}
    >
      <h1 style={{ fontSize: "1.25rem", marginBottom: "1rem" }}>
        API Dev Testing
      </h1>

      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.5rem" }}>
        <select
          value={method}
          onChange={(e) => setMethod(e.target.value as Method)}
          style={inputStyle}
        >
          <option>GET</option>
          <option>POST</option>
          <option>PUT</option>
          <option>DELETE</option>
        </select>
        <input
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="/api/..."
          style={{ ...inputStyle, flex: 1 }}
        />
        <button type="button" onClick={sendRequest} style={buttonStyle}>
          Send
        </button>
      </div>

      {method !== "GET" && (
        <textarea
          value={body}
          onChange={(e) => setBody(e.target.value)}
          placeholder='{"key": "value"}'
          rows={4}
          style={{
            ...inputStyle,
            width: "100%",
            resize: "vertical",
            marginBottom: "0.5rem",
          }}
        />
      )}

      <hr style={{ margin: "1rem 0", borderColor: "#333" }} />

      {history.map((entry) => (
        <div
          key={entry.id}
          style={{
            marginBottom: "1rem",
            padding: "0.75rem",
            border: "1px solid #444",
            borderRadius: "4px",
            background: "#1a1a1a",
            color: "#e0e0e0",
          }}
        >
          <div style={{ marginBottom: "0.25rem" }}>
            <span
              style={{ color: methodColor(entry.method), fontWeight: "bold" }}
            >
              {entry.method}
            </span>{" "}
            {entry.path}
            {entry.pending ? (
              <span style={{ color: "#888" }}> ...</span>
            ) : (
              <span style={{ color: statusColor(entry.status ?? 0) }}>
                {" "}
                {entry.status}
              </span>
            )}
          </div>
          {entry.response !== null && (
            <pre
              style={{
                margin: 0,
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
                fontSize: "0.8rem",
                color: "#ccc",
              }}
            >
              {entry.response}
            </pre>
          )}
        </div>
      ))}
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  fontFamily: "monospace",
  fontSize: "0.9rem",
  padding: "0.4rem 0.6rem",
  border: "1px solid #555",
  borderRadius: "4px",
  background: "#1a1a1a",
  color: "#e0e0e0",
};

const buttonStyle: React.CSSProperties = {
  ...inputStyle,
  cursor: "pointer",
  background: "#2563eb",
  color: "white",
  border: "1px solid #2563eb",
  fontWeight: "bold",
};

function methodColor(m: Method): string {
  switch (m) {
    case "GET":
      return "#4ade80";
    case "POST":
      return "#60a5fa";
    case "PUT":
      return "#fbbf24";
    case "DELETE":
      return "#f87171";
  }
}

function statusColor(s: number): string {
  if (s === 0) return "#888";
  if (s < 300) return "#4ade80";
  if (s < 400) return "#fbbf24";
  return "#f87171";
}
