/**
 * The two backends and how to call them.
 *
 * - The **server** is same-origin; its session cookie (`rs_session`) is sent with every request.
 * - **Trustauth** is a different origin, which the server announces at runtime in
 *   `GET /api/config` — so one build works in development and production alike. Its cookie
 *   (`ta_session`) is sent cross-origin (`credentials: "include"`), which its CORS policy allows
 *   for the server's origins only.
 *
 * Both cookies are `HttpOnly`: page scripts never see them, and nothing else about a login is
 * stored in the browser. Every non-2xx response becomes an {@link ApiError}.
 */

import { ApiError, type ErrorCode } from "./error";

// Set VITE_API_ENDPOINT at build time to serve the API from another origin; by default the API
// is same-origin (and proxied by Vite in development).
const API_BASE = (
  (import.meta.env.VITE_API_ENDPOINT as string | undefined) ?? ""
).replace(/\/$/, "");

export function apiUrl(path: string): string {
  return `${API_BASE}${path}`;
}

async function request<T>(url: string, init: RequestInit): Promise<T> {
  let res: Response;
  try {
    res = await fetch(url, {
      ...init,
      headers: init.body ? { "Content-Type": "application/json" } : undefined,
    });
  } catch {
    throw new ApiError(
      "NetworkError",
      "Could not reach the server. Check your connection.",
      0,
    );
  }

  const text = await res.text();
  // biome-ignore lint/suspicious/noExplicitAny: JSON of unknown shape
  let body: any = null;
  if (text) {
    try {
      body = JSON.parse(text);
    } catch {
      body = null;
    }
  }

  if (!res.ok) {
    const code: ErrorCode =
      body?.code ?? (res.status >= 500 ? "Internal" : "InvalidInput");
    const message: string =
      body?.message ?? `Request failed (HTTP ${res.status}).`;
    throw new ApiError(code, message, res.status);
  }
  return body as T;
}

function json(method: string, body?: unknown): RequestInit {
  return {
    method,
    body: body === undefined ? undefined : JSON.stringify(body),
  };
}

export const server = {
  get: <T>(path: string) =>
    request<T>(apiUrl(path), { credentials: "include" }),
  post: <T>(path: string, body?: unknown) =>
    request<T>(apiUrl(path), {
      ...json("POST", body ?? {}),
      credentials: "include",
    }),
  put: <T>(path: string, body: unknown) =>
    request<T>(apiUrl(path), { ...json("PUT", body), credentials: "include" }),
  delete: <T>(path: string) =>
    request<T>(apiUrl(path), { method: "DELETE", credentials: "include" }),
  /** For `POST /api/ballot`: no cookies at all, so the ballot can't be tied to a session. */
  postAnonymous: <T>(path: string, body: unknown) =>
    request<T>(apiUrl(path), { ...json("POST", body), credentials: "omit" }),
};

/** What `GET /api/config` returns. */
export interface ClientConfig {
  trustauthUrl: string;
  maxNameLength: number;
  maxLabelLength: number;
  maxCandidates: number;
}

let config: Promise<ClientConfig> | null = null;

/** The deployment's configuration, fetched once per page load (and again if that failed). */
export function getConfig(): Promise<ClientConfig> {
  if (!config) {
    config = server.get<ClientConfig>("/api/config");
    config.catch(() => {
      config = null;
    });
  }
  return config;
}

async function trustauthUrl(path: string): Promise<string> {
  const { trustauthUrl: base } = await getConfig();
  return `${base.replace(/\/$/, "")}${path}`;
}

export const trustauth = {
  get: async <T>(path: string) =>
    request<T>(await trustauthUrl(path), { credentials: "include" }),
  post: async <T>(path: string, body?: unknown) =>
    request<T>(await trustauthUrl(path), {
      ...json("POST", body ?? {}),
      credentials: "include",
    }),
};
