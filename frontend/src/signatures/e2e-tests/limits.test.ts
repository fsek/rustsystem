/**
 * @vitest-environment node
 *
 * E2E tests verifying server-side field length limits.
 *
 * For each validated field we run two cases:
 *   - exactly at the limit  → request should succeed (2xx)
 *   - one character over    → 422 Unprocessable Entity
 *
 * Limits are fetched from GET /api/limits before the suite runs so that
 * these tests stay in sync with the server constants automatically.
 *
 * Requires both services running:
 *   Server:    http://localhost:1443  (or override with E2E_API_URL)
 *   Trustauth: http://localhost:2443  (or override with E2E_TRUSTAUTH_URL)
 */

import { TestClient, BASE_URL } from "./helpers";

interface Limits {
  maxNameLength: number;
  maxLabelLength: number;
}

// Fetch limits from the server. Also serves as the service-reachability check.
let limits: Limits | null = null;
try {
  const res = await fetch(`${BASE_URL}/api/limits`);
  if (res.ok) limits = (await res.json()) as Limits;
} catch {
  /* services not running — all suites will be skipped */
}

const servicesReachable = limits !== null;

// ── create-meeting: title ──────────────────────────────────────────────────────

describe.skipIf(!servicesReachable)("field limits — meeting title", () => {
  it(`accepts a meeting title at the limit (${limits?.maxLabelLength} chars)`, async () => {
    const client = new TestClient();
    const res = await client.rawRequest("POST", "/api/create-meeting", {
      title: "x".repeat(limits!.maxLabelLength),
      host_name: "Host",
      pub_key: "key",
    });
    expect(res.ok).toBe(true);
  });

  it(`rejects a meeting title one char over the limit (${(limits?.maxLabelLength ?? 0) + 1} chars)`, async () => {
    const client = new TestClient();
    const res = await client.rawRequest("POST", "/api/create-meeting", {
      title: "x".repeat(limits!.maxLabelLength + 1),
      host_name: "Host",
      pub_key: "key",
    });
    expect(res.status).toBe(422);
  });
});

// ── create-meeting: host name ──────────────────────────────────────────────────

describe.skipIf(!servicesReachable)("field limits — host name", () => {
  it(`accepts a host name at the limit (${limits?.maxNameLength} chars)`, async () => {
    const client = new TestClient();
    const res = await client.rawRequest("POST", "/api/create-meeting", {
      title: "Test Meeting",
      host_name: "x".repeat(limits!.maxNameLength),
      pub_key: "key",
    });
    expect(res.ok).toBe(true);
  });

  it(`rejects a host name one char over the limit (${(limits?.maxNameLength ?? 0) + 1} chars)`, async () => {
    const client = new TestClient();
    const res = await client.rawRequest("POST", "/api/create-meeting", {
      title: "Test Meeting",
      host_name: "x".repeat(limits!.maxNameLength + 1),
      pub_key: "key",
    });
    expect(res.status).toBe(422);
  });
});

// ── new-voter: voter name ──────────────────────────────────────────────────────

describe.skipIf(!servicesReachable)("field limits — voter name", () => {
  it(`accepts a voter name at the limit (${limits?.maxNameLength} chars)`, async () => {
    const client = new TestClient();
    await client.createMeeting();
    const res = await client.rawRequest("POST", "/api/host/new-voter", {
      voterName: "x".repeat(limits!.maxNameLength),
      isHost: false,
    });
    expect(res.ok).toBe(true);
  });

  it(`rejects a voter name one char over the limit (${(limits?.maxNameLength ?? 0) + 1} chars)`, async () => {
    const client = new TestClient();
    await client.createMeeting();
    const res = await client.rawRequest("POST", "/api/host/new-voter", {
      voterName: "x".repeat(limits!.maxNameLength + 1),
      isHost: false,
    });
    expect(res.status).toBe(422);
  });
});

// ── start-vote: round name ─────────────────────────────────────────────────────

describe.skipIf(!servicesReachable)("field limits — vote round name", () => {
  it(`accepts a vote round name at the limit (${limits?.maxLabelLength} chars)`, async () => {
    const client = new TestClient();
    await client.createMeeting();
    const res = await client.rawRequest("POST", "/api/host/start-vote", {
      name: "x".repeat(limits!.maxLabelLength),
      shuffle: false,
      metadata: { candidates: ["A", "B"], max_choices: 1, protocol_version: 1 },
    });
    expect(res.ok).toBe(true);
  });

  it(`rejects a vote round name one char over the limit (${(limits?.maxLabelLength ?? 0) + 1} chars)`, async () => {
    const client = new TestClient();
    await client.createMeeting();
    const res = await client.rawRequest("POST", "/api/host/start-vote", {
      name: "x".repeat(limits!.maxLabelLength + 1),
      shuffle: false,
      metadata: { candidates: ["A", "B"], max_choices: 1, protocol_version: 1 },
    });
    expect(res.status).toBe(422);
  });
});

// ── start-vote: candidate name ─────────────────────────────────────────────────

describe.skipIf(!servicesReachable)("field limits — candidate name", () => {
  it(`accepts candidate names at the limit (${limits?.maxNameLength} chars)`, async () => {
    const client = new TestClient();
    await client.createMeeting();
    const res = await client.rawRequest("POST", "/api/host/start-vote", {
      name: "Test Vote",
      shuffle: false,
      metadata: {
        candidates: ["x".repeat(limits!.maxNameLength), "Other"],
        max_choices: 1,
        protocol_version: 1,
      },
    });
    expect(res.ok).toBe(true);
  });

  it(`rejects a candidate name one char over the limit (${(limits?.maxNameLength ?? 0) + 1} chars)`, async () => {
    const client = new TestClient();
    await client.createMeeting();
    const res = await client.rawRequest("POST", "/api/host/start-vote", {
      name: "Test Vote",
      shuffle: false,
      metadata: {
        candidates: ["x".repeat(limits!.maxNameLength + 1), "Other"],
        max_choices: 1,
        protocol_version: 1,
      },
    });
    expect(res.status).toBe(422);
  });
});
