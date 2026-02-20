/**
 * E2E tests for the production pages: /create-meeting, /admin, /login, /meeting.
 *
 * These tests exercise the real user flows that run in production across real
 * browser engines, covering:
 *
 *   • /create-meeting — form validation, meeting creation, redirect to /admin
 *   • /admin          — voter management, vote round state machine, tally, SSE
 *   • /login          — error states (missing params, 409, 404), host/voter redirects
 *   • /meeting        — voter journey: register → select → submit, blank vote, SSE state
 *
 * Unlike voting.spec.ts (which uses the /signature-dev dev harness), these tests
 * drive the actual production UI that voters and hosts will use.
 *
 * Prerequisites — both must be running before `pnpm test:e2e`:
 *   Rust backend:  API_ENDPOINT=http://localhost:3000 cargo run --bin rustsystem-server
 *   Vite server:   started automatically by playwright.config.ts webServer option
 *
 * Each test creates its own meeting so tests are fully independent and can run
 * in parallel. Multi-context tests (host + voter) use the `browser` fixture to
 * create separate cookie jars. Every test that opens a meeting closes it at the
 * end via closeMeetingFromPage() to keep the server clean.
 */

import { test, expect, type Page } from "@playwright/test";

// ─── Shared helpers ───────────────────────────────────────────────────────────

/**
 * Navigate to /create-meeting and wait for the form to be interactive.
 */
async function gotoCreateMeeting(page: Page) {
  await page.goto("/create-meeting");
  await expect(page.locator("#meeting-title")).toBeVisible();
}

/**
 * Fill the New Meeting form and submit it, waiting for navigation to /admin
 * and for the admin panel to finish its initial data load.
 */
async function createMeetingViaUI(
  page: Page,
  title = "Test Meeting",
  hostName = "Test Host",
) {
  await gotoCreateMeeting(page);
  await page.locator("#meeting-title").fill(title);
  await page.locator("#host-name").fill(hostName);
  await page.getByRole("button", { name: "Create Meeting" }).click();
  await page.waitForURL("**/admin");
  // Wait for the voter list panel to finish loading before proceeding.
  await expect(page.getByText(/^Voters \(/)).toBeVisible();
}

/**
 * From an already-loaded /admin page, add a voter by name and return the
 * invite link extracted from the QR panel. The QR panel is left open so the
 * caller can decide whether to dismiss it.
 */
async function addVoterAndGetInviteLink(
  adminPage: Page,
  name: string,
): Promise<string> {
  await adminPage.getByPlaceholder("Name").fill(name);
  await adminPage.getByRole("button", { name: "Add", exact: true }).click();
  await expect(adminPage.getByText(`Invite — ${name}`)).toBeVisible();
  const link = await adminPage.locator("code").last().innerText();
  return link.trim();
}

/**
 * From /admin, configure and start a vote round, waiting for the state badge
 * to read "Voting open". Fills the first N option slots (max 3 initial slots).
 */
async function startVoteRoundFromAdmin(
  adminPage: Page,
  name: string,
  options: [string, string, ...string[]],
) {
  await adminPage.getByPlaceholder("e.g. Board election").fill(name);
  for (let i = 0; i < Math.min(options.length, 3); i++) {
    await adminPage.getByPlaceholder(`Option ${i + 1}`).fill(options[i]);
  }
  await adminPage.getByRole("button", { name: "Start vote round" }).click();
  await expect(adminPage.getByText("Voting open")).toBeVisible();
}

/**
 * From /admin in Voting state, tally votes and wait until the "Tally votes"
 * button disappears (indicating the state has transitioned to Tally).
 */
async function tallyFromAdmin(adminPage: Page) {
  await adminPage.getByRole("button", { name: "Tally votes" }).click();
  await expect(
    adminPage.getByRole("button", { name: "Tally votes" }),
  ).not.toBeVisible();
}

/**
 * From /admin in Voting or Tally state, end the round and wait for the state
 * badge to return to "Idle".
 */
async function endRoundFromAdmin(adminPage: Page) {
  await adminPage.getByRole("button", { name: "End round" }).click();
  await expect(adminPage.getByText("Idle")).toBeVisible();
}

/**
 * Close the current meeting via the host API. Call this at the end of every
 * test that creates a meeting so that the server stays clean between runs.
 */
async function closeMeetingFromPage(page: Page): Promise<void> {
  await page.request.fetch("/api/host/close-meeting", { method: "DELETE" });
}

// ─── /create-meeting ──────────────────────────────────────────────────────────

test.describe("/create-meeting", () => {
  test("renders the meeting title and host name fields", async ({ page }) => {
    await gotoCreateMeeting(page);
    await expect(page.locator("#meeting-title")).toBeVisible();
    await expect(page.locator("#host-name")).toBeVisible();
    // No meeting was created — nothing to close.
  });

  test("submit button is disabled when both fields are empty", async ({
    page,
  }) => {
    await gotoCreateMeeting(page);
    await expect(
      page.getByRole("button", { name: "Create Meeting" }),
    ).toBeDisabled();
  });

  test("submit button is disabled when only meeting title is filled", async ({
    page,
  }) => {
    await gotoCreateMeeting(page);
    await page.locator("#meeting-title").fill("Annual GM");
    await expect(
      page.getByRole("button", { name: "Create Meeting" }),
    ).toBeDisabled();
  });

  test("submit button is disabled when only host name is filled", async ({
    page,
  }) => {
    await gotoCreateMeeting(page);
    await page.locator("#host-name").fill("Alice");
    await expect(
      page.getByRole("button", { name: "Create Meeting" }),
    ).toBeDisabled();
  });

  test("successful submission navigates to /admin", async ({ page }) => {
    await createMeetingViaUI(page, "Annual GM 2026", "Alice");
    await expect(page).toHaveURL(/\/admin/);
    await closeMeetingFromPage(page);
  });

  test("host appears in the voter list after creation", async ({ page }) => {
    await createMeetingViaUI(page, "Spring GM", "Alice");
    // The host is automatically added as the first voter.
    await expect(page.getByText("Alice")).toBeVisible();
    await closeMeetingFromPage(page);
  });

  test("host voter carries a 'host' badge in the voter list", async ({
    page,
  }) => {
    await createMeetingViaUI(page, "Meeting", "Alice");
    // The host row shows a "host" badge next to the name.
    await expect(page.getByText("host", { exact: true })).toBeVisible();
    await closeMeetingFromPage(page);
  });
});

// ─── /admin — voter management ────────────────────────────────────────────────

test.describe("/admin — voter management", () => {
  test("shows 'Idle' vote state badge on initial load", async ({ page }) => {
    await createMeetingViaUI(page);
    await expect(page.getByText("Idle")).toBeVisible();
    await closeMeetingFromPage(page);
  });

  test("unauthenticated access shows a load error alert", async ({ page }) => {
    // Navigating directly without a session cookie causes the API calls to fail.
    await page.goto("/admin");
    await expect(page.getByRole("alert")).toBeVisible();
    // No meeting was created — nothing to close.
  });

  test("adding a voter shows the QR panel with a valid invite link", async ({
    page,
  }) => {
    await createMeetingViaUI(page);
    await page.getByPlaceholder("Name").fill("Bob");
    await page.getByRole("button", { name: "Add", exact: true }).click();
    await expect(page.getByText("Invite — Bob")).toBeVisible();
    const link = await page.locator("code").last().innerText();
    expect(link).toMatch(/\/login\?/);
    expect(link).toMatch(/muuid=/);
    expect(link).toMatch(/uuuid=/);
    await closeMeetingFromPage(page);
  });

  test("dismissing the QR panel removes it from view", async ({ page }) => {
    await createMeetingViaUI(page);
    await page.getByPlaceholder("Name").fill("Carol");
    await page.getByRole("button", { name: "Add", exact: true }).click();
    await expect(page.getByText("Invite — Carol")).toBeVisible();
    await page.getByRole("button", { name: "Dismiss" }).click();
    await expect(page.getByText("Invite — Carol")).not.toBeVisible();
    await closeMeetingFromPage(page);
  });

  test("added voter appears in the voter list", async ({ page }) => {
    await createMeetingViaUI(page);
    await addVoterAndGetInviteLink(page, "Dave");
    await page.getByRole("button", { name: "Dismiss" }).click();
    await expect(page.getByText("Dave")).toBeVisible();
    await closeMeetingFromPage(page);
  });

  test("removing a voter removes them from the list", async ({ page }) => {
    await createMeetingViaUI(page);
    await addVoterAndGetInviteLink(page, "Eve");
    await page.getByRole("button", { name: "Dismiss" }).click();
    await expect(page.getByText("Eve")).toBeVisible();
    // Scope the × button to Eve's voter row. The row div contains both the
    // "Eve" span and the × button; navigating via the span's parent avoids
    // matching the VoteOptionsInput × buttons elsewhere on the page.
    await page
      .getByText("Eve", { exact: true })
      .locator("..")
      .getByRole("button", { name: "×" })
      .click();
    await expect(page.getByText("Eve")).not.toBeVisible();
    await closeMeetingFromPage(page);
  });

  test("'Remove all' clears every non-host voter", async ({ page }) => {
    await createMeetingViaUI(page, "GM", "Alice");
    await addVoterAndGetInviteLink(page, "Bob");
    await page.getByRole("button", { name: "Dismiss" }).click();
    await addVoterAndGetInviteLink(page, "Carol");
    await page.getByRole("button", { name: "Dismiss" }).click();
    await expect(page.getByText("Bob")).toBeVisible();
    await expect(page.getByText("Carol")).toBeVisible();
    await page.getByRole("button", { name: "Remove all" }).click();
    await expect(page.getByText("Bob")).not.toBeVisible();
    await expect(page.getByText("Carol")).not.toBeVisible();
    // The host should still be present.
    await expect(page.getByText("Alice")).toBeVisible();
    await closeMeetingFromPage(page);
  });

  test("adding a voter with host privileges shows 'host' badge in voter list", async ({
    page,
  }) => {
    await createMeetingViaUI(page);
    await page.getByPlaceholder("Name").fill("Co-Host");
    await page.getByLabel("Grant host privileges").check();
    await page.getByRole("button", { name: "Add", exact: true }).click();
    await page.getByRole("button", { name: "Dismiss" }).click();
    // Two rows should now carry the "host" badge.
    await expect(page.getByText("host", { exact: true })).toHaveCount(2);
    await closeMeetingFromPage(page);
  });
});

// ─── /admin — vote round state machine ────────────────────────────────────────

test.describe("/admin — vote round state machine", () => {
  test("starting a vote round transitions the badge to 'Voting open'", async ({
    page,
  }) => {
    await createMeetingViaUI(page);
    await startVoteRoundFromAdmin(page, "Board Election", ["Alice", "Bob"]);
    await expect(page.getByText("Voting open")).toBeVisible();
    await closeMeetingFromPage(page);
  });

  test("ending a round without tallying resets the badge to 'Idle'", async ({
    page,
  }) => {
    await createMeetingViaUI(page);
    await startVoteRoundFromAdmin(page, "Test Vote", ["Yes", "No"]);
    await endRoundFromAdmin(page);
    await expect(page.getByText("Idle")).toBeVisible();
    // The creation form must be available again.
    await expect(
      page.getByRole("button", { name: "Start vote round" }),
    ).toBeVisible();
    await closeMeetingFromPage(page);
  });

  test("tallying shows candidate results and an 'End round' button", async ({
    page,
  }) => {
    await createMeetingViaUI(page);
    await startVoteRoundFromAdmin(page, "Resolution A", ["For", "Against"]);
    await tallyFromAdmin(page);
    // Both candidates must appear in the results bars.
    await expect(page.getByText("For")).toBeVisible();
    await expect(page.getByText("Against")).toBeVisible();
    await expect(page.getByRole("button", { name: "End round" })).toBeVisible();
    await closeMeetingFromPage(page);
  });

  test("ending after tally resets the badge to 'Idle'", async ({ page }) => {
    await createMeetingViaUI(page);
    await startVoteRoundFromAdmin(page, "Resolution B", ["Yes", "No"]);
    await tallyFromAdmin(page);
    await endRoundFromAdmin(page);
    await expect(page.getByText("Idle")).toBeVisible();
    await closeMeetingFromPage(page);
  });

  test("vote name is displayed in the Voting panel", async ({ page }) => {
    await createMeetingViaUI(page);
    await startVoteRoundFromAdmin(page, "Annual Budget Vote", [
      "Approve",
      "Reject",
    ]);
    // The vote name appears in both HostVoteRoundPanel and the host's VotePanel.
    await expect(page.getByText("Annual Budget Vote").first()).toBeVisible();
    await closeMeetingFromPage(page);
  });

  test("a second round can be started after the first ends", async ({
    page,
  }) => {
    await createMeetingViaUI(page);
    await startVoteRoundFromAdmin(page, "Round 1", ["Yes", "No"]);
    await endRoundFromAdmin(page);
    await startVoteRoundFromAdmin(page, "Round 2", ["Option A", "Option B"]);
    await expect(page.getByText("Voting open")).toBeVisible();
    await expect(page.getByText("Round 2").first()).toBeVisible();
    await closeMeetingFromPage(page);
  });

  test("tally download button opens a format dropdown", async ({ page }) => {
    await createMeetingViaUI(page);
    await startVoteRoundFromAdmin(page, "Vote", ["Yes", "No"]);
    await tallyFromAdmin(page);
    await page.locator('button[title="Download Results"]').click();
    await expect(
      page.getByRole("button", { name: "JSON", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "YAML", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "TOML", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "RON", exact: true }),
    ).toBeVisible();
    await closeMeetingFromPage(page);
  });

  test("vote progress bar is shown while voting is active", async ({
    page,
  }) => {
    await createMeetingViaUI(page);
    await addVoterAndGetInviteLink(page, "Voter1");
    await page.getByRole("button", { name: "Dismiss" }).click();
    await startVoteRoundFromAdmin(page, "Vote", ["Yes", "No"]);
    // Progress row shows "Votes cast" label and a fraction.
    await expect(page.getByText("Votes cast")).toBeVisible();
    await closeMeetingFromPage(page);
  });
});

// ─── /login ───────────────────────────────────────────────────────────────────

test.describe("/login", () => {
  test("shows error immediately when muuid and uuuid params are missing", async ({
    page,
  }) => {
    await page.goto("/login");
    await expect(
      page.getByRole("alert").filter({ hasText: "Invalid login link" }),
    ).toBeVisible();
    // No meeting was created — nothing to close.
  });

  test("shows error for a non-existent meeting", async ({ page }) => {
    // Nil UUIDs will not match any meeting in the server.
    await page.goto(
      "/login?muuid=00000000-0000-0000-0000-000000000000&uuuid=00000000-0000-0000-0000-000000000001",
    );
    await expect(
      page.getByRole("alert").filter({ hasText: /not found|failed/i }),
    ).toBeVisible();
  });

  test("valid voter invite link logs in and redirects to /meeting", async ({
    browser,
  }) => {
    const hostCtx = await browser.newContext();
    const hostPage = await hostCtx.newPage();
    await createMeetingViaUI(hostPage);
    const inviteLink = await addVoterAndGetInviteLink(hostPage, "Grace");

    const voterCtx = await browser.newContext();
    const voterPage = await voterCtx.newPage();
    await voterPage.goto(inviteLink);
    await voterPage.waitForURL("**/meeting");
    await expect(voterPage).toHaveURL(/\/meeting/);

    await closeMeetingFromPage(hostPage);
    await hostCtx.close();
    await voterCtx.close();
  });

  test("reusing an invite link shows a 409 'already been used' error", async ({
    browser,
  }) => {
    const hostCtx = await browser.newContext();
    const hostPage = await hostCtx.newPage();
    await createMeetingViaUI(hostPage);
    const inviteLink = await addVoterAndGetInviteLink(hostPage, "Frank");

    // First use succeeds.
    const voterCtx1 = await browser.newContext();
    const voterPage1 = await voterCtx1.newPage();
    await voterPage1.goto(inviteLink);
    await voterPage1.waitForURL("**/meeting");

    // Second use must fail with the "already been used" message.
    const voterCtx2 = await browser.newContext();
    const voterPage2 = await voterCtx2.newPage();
    await voterPage2.goto(inviteLink);
    await expect(
      voterPage2.getByRole("alert").filter({ hasText: "already been used" }),
    ).toBeVisible();

    await closeMeetingFromPage(hostPage);
    await hostCtx.close();
    await voterCtx1.close();
    await voterCtx2.close();
  });
});

// ─── /meeting ─────────────────────────────────────────────────────────────────

test.describe("/meeting", () => {
  test("shows 'Waiting for voting to start' when no round is active", async ({
    browser,
  }) => {
    const hostCtx = await browser.newContext();
    const hostPage = await hostCtx.newPage();
    await createMeetingViaUI(hostPage);
    const inviteLink = await addVoterAndGetInviteLink(hostPage, "Ivan");

    const voterCtx = await browser.newContext();
    const voterPage = await voterCtx.newPage();
    await voterPage.goto(inviteLink);
    await voterPage.waitForURL("**/meeting");

    await expect(
      voterPage.getByText("Waiting for voting to start…"),
    ).toBeVisible();

    await closeMeetingFromPage(hostPage);
    await hostCtx.close();
    await voterCtx.close();
  });

  test("'Register to vote' appears when the host starts a round (SSE update)", async ({
    browser,
  }) => {
    const hostCtx = await browser.newContext();
    const hostPage = await hostCtx.newPage();
    await createMeetingViaUI(hostPage);
    const inviteLink = await addVoterAndGetInviteLink(hostPage, "Judy");

    const voterCtx = await browser.newContext();
    const voterPage = await voterCtx.newPage();
    await voterPage.goto(inviteLink);
    await voterPage.waitForURL("**/meeting");

    // Voter is on /meeting and no round is active yet.
    await expect(
      voterPage.getByText("Waiting for voting to start…"),
    ).toBeVisible();

    // Host starts the round — the voter's page must update via SSE.
    await startVoteRoundFromAdmin(hostPage, "Test Vote", [
      "Option A",
      "Option B",
    ]);

    await expect(
      voterPage.getByRole("button", { name: "Register to vote" }),
    ).toBeVisible();

    await closeMeetingFromPage(hostPage);
    await hostCtx.close();
    await voterCtx.close();
  });

  test("'Submit vote' button is disabled until a candidate is selected", async ({
    browser,
  }) => {
    const hostCtx = await browser.newContext();
    const hostPage = await hostCtx.newPage();
    await createMeetingViaUI(hostPage);
    const inviteLink = await addVoterAndGetInviteLink(hostPage, "Mike");

    // Voter must log in before the vote starts — non-logged-in voters are
    // removed from the meeting when the round begins.
    const voterCtx = await browser.newContext();
    const voterPage = await voterCtx.newPage();
    await voterPage.goto(inviteLink);
    await voterPage.waitForURL("**/meeting");

    await startVoteRoundFromAdmin(hostPage, "Vote", ["Yes", "No"]);

    await voterPage.getByRole("button", { name: "Register to vote" }).click();
    // Immediately after registration, no candidate is selected.
    await expect(
      voterPage.getByRole("button", { name: "Submit vote" }),
    ).toBeDisabled();

    // Selecting a candidate enables the submit button.
    await voterPage.getByRole("button", { name: "Yes" }).click();
    await expect(
      voterPage.getByRole("button", { name: "Submit vote" }),
    ).toBeEnabled();

    await closeMeetingFromPage(hostPage);
    await hostCtx.close();
    await voterCtx.close();
  });

  test("full voter journey: register → select candidate → submit", async ({
    browser,
  }) => {
    const hostCtx = await browser.newContext();
    const hostPage = await hostCtx.newPage();
    await createMeetingViaUI(hostPage);
    const inviteLink = await addVoterAndGetInviteLink(hostPage, "Karl");

    const voterCtx = await browser.newContext();
    const voterPage = await voterCtx.newPage();
    await voterPage.goto(inviteLink);
    await voterPage.waitForURL("**/meeting");

    await startVoteRoundFromAdmin(hostPage, "Board Vote", ["Alice", "Bob"]);

    await voterPage.getByRole("button", { name: "Register to vote" }).click();
    await expect(
      voterPage.getByRole("button", { name: "Alice" }),
    ).toBeVisible();
    await voterPage.getByRole("button", { name: "Alice" }).click();
    await voterPage.getByRole("button", { name: "Submit vote" }).click();

    await expect(
      voterPage.getByRole("alert").filter({ hasText: "submitted anonymously" }),
    ).toBeVisible();

    await closeMeetingFromPage(hostPage);
    await hostCtx.close();
    await voterCtx.close();
  });

  test("blank vote shows the confirmation alert", async ({ browser }) => {
    const hostCtx = await browser.newContext();
    const hostPage = await hostCtx.newPage();
    await createMeetingViaUI(hostPage);
    const inviteLink = await addVoterAndGetInviteLink(hostPage, "Laura");

    const voterCtx = await browser.newContext();
    const voterPage = await voterCtx.newPage();
    await voterPage.goto(inviteLink);
    await voterPage.waitForURL("**/meeting");

    await startVoteRoundFromAdmin(hostPage, "Resolution", ["For", "Against"]);

    await voterPage.getByRole("button", { name: "Register to vote" }).click();
    await expect(
      voterPage.getByRole("button", { name: "Blank vote" }),
    ).toBeVisible();
    await voterPage.getByRole("button", { name: "Blank vote" }).click();

    await expect(
      voterPage.getByRole("alert").filter({ hasText: "submitted anonymously" }),
    ).toBeVisible();

    await closeMeetingFromPage(hostPage);
    await hostCtx.close();
    await voterCtx.close();
  });

  test("voter sees 'The voting is now over' after the host tallies", async ({
    browser,
  }) => {
    const hostCtx = await browser.newContext();
    const hostPage = await hostCtx.newPage();
    await createMeetingViaUI(hostPage);
    const inviteLink = await addVoterAndGetInviteLink(hostPage, "Nina");

    const voterCtx = await browser.newContext();
    const voterPage = await voterCtx.newPage();
    await voterPage.goto(inviteLink);
    await voterPage.waitForURL("**/meeting");

    await startVoteRoundFromAdmin(hostPage, "Final Vote", ["Pass", "Fail"]);

    // Voter submits a blank vote.
    await voterPage.getByRole("button", { name: "Register to vote" }).click();
    await voterPage.getByRole("button", { name: "Blank vote" }).click();
    await expect(
      voterPage.getByRole("alert").filter({ hasText: "submitted anonymously" }),
    ).toBeVisible();

    // Host tallies — voter's page must update via SSE.
    await tallyFromAdmin(hostPage);

    await expect(voterPage.getByText("The voting is now over.")).toBeVisible();

    await closeMeetingFromPage(hostPage);
    await hostCtx.close();
    await voterCtx.close();
  });
});

// ─── /admin — voter login notification ────────────────────────────────────────

test.describe("/admin — voter login notification", () => {
  test("shows '<name> has logged in.' after the voter follows the invite link", async ({
    browser,
  }) => {
    const hostCtx = await browser.newContext();
    const hostPage = await hostCtx.newPage();
    await createMeetingViaUI(hostPage);
    const inviteLink = await addVoterAndGetInviteLink(hostPage, "Oscar");

    const voterCtx = await browser.newContext();
    const voterPage = await voterCtx.newPage();
    await voterPage.goto(inviteLink);
    await voterPage.waitForURL("**/meeting");

    // The admin page receives the login event over SSE and shows the alert.
    await expect(
      hostPage.getByRole("alert").filter({ hasText: "Oscar has logged in." }),
    ).toBeVisible();

    await closeMeetingFromPage(hostPage);
    await hostCtx.close();
    await voterCtx.close();
  });

  test("voter login notification can be dismissed with the × button", async ({
    browser,
  }) => {
    const hostCtx = await browser.newContext();
    const hostPage = await hostCtx.newPage();
    await createMeetingViaUI(hostPage);
    const inviteLink = await addVoterAndGetInviteLink(hostPage, "Penny");

    const voterCtx = await browser.newContext();
    const voterPage = await voterCtx.newPage();
    await voterPage.goto(inviteLink);
    await voterPage.waitForURL("**/meeting");

    const loginAlert = hostPage
      .getByRole("alert")
      .filter({ hasText: "Penny has logged in." });
    await expect(loginAlert).toBeVisible();

    // The Alert's built-in dismiss button carries aria-label="Dismiss".
    await loginAlert.getByRole("button", { name: "Dismiss" }).click();
    await expect(loginAlert).not.toBeVisible();

    await closeMeetingFromPage(hostPage);
    await hostCtx.close();
    await voterCtx.close();
  });

  test("voter list reloads and voter is visible after login", async ({
    browser,
  }) => {
    // The SSE "Ready" handler calls reloadVoters() when any voter logs in.
    // Verify the voter remains visible in the list after the QR panel is
    // dismissed and the login notification is cleared.
    const hostCtx = await browser.newContext();
    const hostPage = await hostCtx.newPage();
    await createMeetingViaUI(hostPage);
    const inviteLink = await addVoterAndGetInviteLink(hostPage, "Quinn");
    // Leave QR panel open so the SSE handler can resolve Quinn's name.

    const voterCtx = await browser.newContext();
    const voterPage = await voterCtx.newPage();
    await voterPage.goto(inviteLink);
    await voterPage.waitForURL("**/meeting");

    // Login notification confirms the SSE fired and the voter list reloaded.
    await expect(
      hostPage.getByRole("alert").filter({ hasText: "Quinn has logged in." }),
    ).toBeVisible();

    // Dismiss the notification and the QR panel.
    await hostPage
      .getByRole("alert")
      .filter({ hasText: "Quinn has logged in." })
      .getByRole("button", { name: "Dismiss" })
      .click();

    // Quinn must still appear in the voter list after the alert is gone.
    await expect(hostPage.getByText("Quinn")).toBeVisible();

    await closeMeetingFromPage(hostPage);
    await hostCtx.close();
    await voterCtx.close();
  });
});
