/**
 * Cross-browser e2e tests for the anonymous voting workflow.
 *
 * These tests exercise the full stack in real browser engines (Chromium,
 * Firefox, WebKit) plus mobile viewports, covering behaviour that the
 * Node.js Vitest e2e tests cannot reach:
 *
 *   • Actual browser cookie jars (HttpOnly / SameSite enforcement)
 *   • localStorage persistence across page reloads
 *   • Crypto in the browser's own JS engine (V8, SpiderMonkey, JavaScriptCore)
 *   • Mobile viewport rendering and touch interaction
 *   • CORS preflight handling (browsers enforce this; Node.js fetch does not)
 *
 * Prerequisites — both must be running before `pnpm test:e2e`:
 *   Rust backend:  API_ENDPOINT=http://localhost:3000 cargo run --bin rustsystem-server
 *   Vite server:   started automatically by playwright.config.ts webServer option
 *
 * Each test creates its own meeting through the UI so tests are fully
 * independent and can run in parallel.
 */

import { test, expect, type Page } from "@playwright/test";

// ─── Shared helpers ───────────────────────────────────────────────────────────

/**
 * Navigate to the dev page and wait until it is interactive.
 * All four step-cards are rendered unconditionally, so we just wait for the
 * Create Meeting button to be visible.
 */
async function goto(page: Page) {
  await page.goto("/signature-dev");
  await page.getByTestId("btn-create-meeting").waitFor({ state: "visible" });
}

/**
 * Click "Create Meeting" and wait for the server to respond.
 * Returns when the success alert ("Meeting ready") becomes visible.
 */
async function createMeeting(page: Page) {
  await page.getByTestId("btn-create-meeting").click();
  await expect(
    page.getByRole("alert").filter({ hasText: "Meeting ready" }),
  ).toBeVisible();
}

/**
 * Click "Start Vote" and wait for the "Vote round active" confirmation.
 */
async function startVote(page: Page) {
  await page.getByTestId("btn-start-vote").click();
  await expect(
    page.getByRole("alert").filter({ hasText: "Vote round active" }),
  ).toBeVisible();
}

/**
 * Click "Register" and wait for the "Blind signature received" confirmation.
 * The BBS+ commitment generation and the round-trip to the server take a
 * moment, especially in Firefox and WebKit — the 10 s expect timeout in the
 * config covers this.
 */
async function register(page: Page) {
  await page.getByTestId("btn-register").click();
  await expect(
    page.getByRole("alert").filter({ hasText: "Blind signature received" }),
  ).toBeVisible();
}

/**
 * Fill the choice input, click "Submit Vote", and wait for the acceptance
 * confirmation.
 *
 * @param choice Comma-separated indices, e.g. "0" or "0,1". Pass "" for blank.
 */
async function submitVote(page: Page, choice: string) {
  await page.getByTestId("input-choice").fill(choice);
  await page.getByTestId("btn-submit").click();
  await expect(
    page.getByRole("alert").filter({ hasText: "Vote accepted" }),
  ).toBeVisible();
}

// ─── Full vote cycle ──────────────────────────────────────────────────────────

test.describe("full vote cycle", () => {
  test("candidate vote (choice index 0)", async ({ page }) => {
    // The canonical happy path: host creates a meeting, starts a round, the
    // voter registers and submits a candidate vote.
    await goto(page);
    await createMeeting(page);
    await startVote(page);
    await register(page);
    await submitVote(page, "0");

    // After a successful submission the token must be cleared from localStorage
    // so it cannot be reused on a subsequent visit.
    const stored = await page.evaluate(() =>
      localStorage.getItem("fsek-vote-session"),
    );
    expect(stored).toBeNull();
  });

  test("blank vote (empty choice)", async ({ page }) => {
    // Voters may abstain by leaving the choice field empty. The server records
    // this separately in the blank counter.
    await goto(page);
    await createMeeting(page);
    await startVote(page);
    await register(page);
    await submitVote(page, ""); // empty → blank vote
  });
});

// ─── localStorage persistence ─────────────────────────────────────────────────

test.describe("localStorage persistence", () => {
  test("token survives a hard page reload", async ({ page }) => {
    // If the voter closes the tab after registration but before voting, the
    // token must still be available on the next visit so they can still submit.
    // voteSession.ts persists the token to localStorage for exactly this reason.
    await goto(page);
    await createMeeting(page);
    await startVote(page);
    await register(page);

    // Force a full navigation (not a soft client-side reload) to simulate
    // the user closing and reopening the browser tab.
    await page.reload();
    await page.getByTestId("btn-create-meeting").waitFor({ state: "visible" });

    // The "restored from storage" banner must appear, indicating that the token
    // was successfully read back from localStorage.
    await expect(page.getByTestId("alert-restored")).toBeVisible();

    // The Submit Vote button must be enabled — registration state is restored.
    await expect(page.getByTestId("btn-submit")).toBeEnabled();

    // The voter can still submit without re-registering.
    await submitVote(page, "0");
  });

  test("token is cleared from localStorage after successful submission", async ({
    page,
  }) => {
    // Once the vote is cast the token is spent and must be removed from storage
    // so it cannot be reused, even if the user navigates back to the page.
    await goto(page);
    await createMeeting(page);
    await startVote(page);
    await register(page);
    await submitVote(page, "0");

    const stored = await page.evaluate(() =>
      localStorage.getItem("fsek-vote-session"),
    );
    expect(stored).toBeNull();

    // After a reload the restored-from-storage banner must NOT appear and the
    // Submit button must be disabled (no token in storage).
    await page.reload();
    await page.getByTestId("btn-create-meeting").waitFor({ state: "visible" });
    await expect(page.getByTestId("alert-restored")).not.toBeVisible();
    await expect(page.getByTestId("btn-submit")).toBeDisabled();
  });

  test("Clear token button removes the token from localStorage", async ({
    page,
  }) => {
    // The "Clear token" button is the escape hatch for shared/public computers:
    // the voter can explicitly discard their token before leaving the machine.
    await goto(page);
    await createMeeting(page);
    await startVote(page);
    await register(page);

    // Token is present in storage after registration.
    const beforeClear = await page.evaluate(() =>
      localStorage.getItem("fsek-vote-session"),
    );
    expect(beforeClear).not.toBeNull();

    await page.getByTestId("btn-clear-token").click();

    const afterClear = await page.evaluate(() =>
      localStorage.getItem("fsek-vote-session"),
    );
    expect(afterClear).toBeNull();

    // The Submit button should now be disabled — no token, no vote.
    await expect(page.getByTestId("btn-submit")).toBeDisabled();
  });
});

// ─── Cookie handling ──────────────────────────────────────────────────────────

test.describe("cookie handling", () => {
  test("session cookie is set after createMeeting and used in subsequent requests", async ({
    page,
    context,
  }) => {
    // The server sets an HttpOnly JWT cookie on the create-meeting response.
    // The browser must attach it automatically on every subsequent API call.
    // This test verifies the cookie jar is populated by checking that Start Vote
    // (which requires an authenticated host session) succeeds after creating a
    // meeting — if the cookie were missing, Start Vote would return 401.
    await goto(page);
    await createMeeting(page);

    // Confirm the cookie is present in the browser's cookie jar.
    const cookies = await context.cookies();
    const sessionCookie = cookies.find((c) => c.name === "access_token");
    expect(sessionCookie).toBeDefined();

    // HttpOnly flag ensures JS cannot read the cookie, but the browser still
    // sends it. Verify the end-to-end flow works (Start Vote succeeds).
    await startVote(page);
  });

  test("session persists across a same-origin navigation", async ({
    page,
    context,
  }) => {
    // Navigating to a different route and back must not lose the session cookie
    // (SameSite=Lax / Strict should allow same-site navigations).
    await goto(page);
    await createMeeting(page);

    // Navigate away (to the root) and come back.
    // waitUntil:"networkidle" ensures TanStack Router fully settles before we
    // fire the next goto — without it, WebKit can raise "navigation interrupted
    // by another navigation" because the SPA's history manipulation is still
    // in flight when the second goto begins.
    await page.goto("/", { waitUntil: "networkidle" });
    await page.goto("/signature-dev");
    await page.getByTestId("btn-create-meeting").waitFor({ state: "visible" });

    // Cookie must still be present after the round-trip navigation.
    const cookies = await context.cookies();
    expect(cookies.find((c) => c.name === "access_token")).toBeDefined();
  });
});

// ─── UI state machine ─────────────────────────────────────────────────────────

test.describe("UI state machine", () => {
  test("Start Vote and Register buttons are disabled until their prerequisites are met", async ({
    page,
  }) => {
    // The UI gates each step on the previous one completing successfully.
    // This prevents confusing API errors from out-of-order actions.
    await goto(page);

    // Before creating a meeting, Start Vote must be disabled.
    await expect(page.getByTestId("btn-start-vote")).toBeDisabled();

    // After creating a meeting, Start Vote becomes enabled.
    await createMeeting(page);
    await expect(page.getByTestId("btn-start-vote")).toBeEnabled();

    // Register must be disabled until a vote round is active.
    await expect(page.getByTestId("btn-register")).toBeDisabled();

    await startVote(page);
    await expect(page.getByTestId("btn-register")).toBeEnabled();
  });

  test("Submit Vote button is disabled until registration is complete", async ({
    page,
  }) => {
    await goto(page);
    await expect(page.getByTestId("btn-submit")).toBeDisabled();

    await createMeeting(page);
    await startVote(page);
    await expect(page.getByTestId("btn-submit")).toBeDisabled();

    await register(page);
    await expect(page.getByTestId("btn-submit")).toBeEnabled();
  });

  test("End Vote Round resets the vote-round state", async ({ page }) => {
    // After ending a round the UI should no longer show "Vote round active"
    // and Register should be disabled again.
    await goto(page);
    await createMeeting(page);
    await startVote(page);

    await page.getByTestId("btn-end-round").click();
    await expect(
      page.getByRole("alert").filter({ hasText: "Round ended" }),
    ).toBeVisible();

    // Register must be disabled again — no active round.
    await expect(page.getByTestId("btn-register")).toBeDisabled();
  });
});

// ─── Mobile viewport smoke tests ─────────────────────────────────────────────

test.describe("mobile viewport", () => {
  test("full vote cycle completes on a mobile viewport", async ({ page }) => {
    // Ensures buttons are tappable and the layout does not break on small
    // screens. The Pixel 7 / iPhone 14 device profiles set a narrow viewport
    // (~390–412 px) and a mobile user-agent string.
    //
    // This is a smoke test; the behaviour is identical to the desktop cycle.
    await goto(page);
    await createMeeting(page);
    await startVote(page);
    await register(page);
    await submitVote(page, "0");
  });
});
