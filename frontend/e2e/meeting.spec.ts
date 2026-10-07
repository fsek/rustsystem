/**
 * A whole meeting in real browsers, through the production pages: a host and a voter, each
 * with their own cookie jar, exactly as two people on two devices.
 *
 * Covers what only a real browser can: HttpOnly/SameSite cookies across the server and
 * trustauth origins, CORS, WebCrypto blind signatures in each engine, refresh behaviour, and
 * decrypting the tally with the meeting password.
 */

import { type Browser, type Page, expect, test } from "@playwright/test";

const PASSWORD = "correct horse battery staple";

async function createMeeting(browser: Browser): Promise<Page> {
  const host = await (await browser.newContext()).newPage();
  await host.goto("/create-meeting");
  await host.getByPlaceholder("e.g. Annual General Meeting").fill("Vårmöte");
  await host.getByPlaceholder("e.g. Jane Smith").fill("Host");
  await host.getByPlaceholder("Use a strong password").fill(PASSWORD);
  await host.getByPlaceholder("Repeat your password").fill(PASSWORD);
  await host.locator("button[type=submit]").click();
  await expect(host).toHaveURL(/\/admin/, { timeout: 30_000 });
  await expect(host.getByRole("heading", { name: "Admin" })).toBeVisible();
  return host;
}

/** Host adds a voter and returns the invite link shown in the QR panel. */
async function invite(host: Page, name: string): Promise<string> {
  await host.getByPlaceholder("Name", { exact: true }).fill(name);
  await host.getByRole("button", { name: "Add", exact: true }).click();
  const link = host.locator("code").filter({ hasText: "/login?meeting=" });
  await expect(link).toBeVisible();
  return (await link.textContent()) ?? "";
}

async function join(browser: Browser, link: string): Promise<Page> {
  const voter = await (await browser.newContext()).newPage();
  await voter.goto(new URL(link).pathname + new URL(link).search);
  await expect(voter).toHaveURL(/\/meeting/);
  await expect(voter.getByText("Waiting for voting to start…")).toBeVisible();
  return voter;
}

async function startRound(host: Page, name: string, options: string[]) {
  await host.getByPlaceholder("e.g. Board election").fill(name);
  for (const [i, option] of options.entries()) {
    await host.getByPlaceholder(`Option ${i + 1}`).fill(option);
  }
  await host.getByRole("button", { name: "Start vote round" }).click();
  await expect(host.getByText("Voting open")).toBeVisible();
}

test("a whole meeting: invite, vote, refresh, tally, decrypt, close", async ({
  browser,
}) => {
  const host = await createMeeting(browser);

  const link = await invite(host, "Anna");
  expect(link).not.toContain("admin");
  const voter = await join(browser, link);
  await expect(host.getByText("Anna has logged in.")).toBeVisible();

  // An invite works once.
  const intruder = await (await browser.newContext()).newPage();
  await intruder.goto(new URL(link).pathname + new URL(link).search);
  await expect(intruder.getByText(/not valid/)).toBeVisible();

  await startRound(host, "Chair", ["Anna", "Bo", "Cecilia"]);

  // The voter's page notices the round by itself.
  await expect(voter.getByText("Select one option.")).toBeVisible();
  await voter.getByRole("button", { name: "Bo" }).click();
  await voter.getByRole("button", { name: "Submit vote" }).click();
  await expect(
    voter.getByText("Your vote has been submitted anonymously."),
  ).toBeVisible();

  // Refreshing doesn't lose anything: still logged in, and it knows we voted.
  await voter.reload();
  await expect(
    voter.getByText("You have already voted in this round."),
  ).toBeVisible();

  // The host votes blank from the admin page.
  await host.getByRole("button", { name: "Blank vote" }).click();
  await expect(
    host.getByText("Your vote has been submitted anonymously."),
  ).toBeVisible();
  await expect(host.getByText("2 / 2")).toBeVisible();

  await host.getByRole("button", { name: "Tally votes" }).click();
  await expect(host.getByText("Blank votes: 1")).toBeVisible();
  await expect(host.getByText(/signed but never arrived/)).toHaveCount(0);
  await expect(voter.getByText("The voting is now over.")).toBeVisible();

  // Decrypt every tally with the meeting password, in the browser.
  await host.getByRole("button", { name: "Close meeting" }).click();
  await host.getByPlaceholder("Meeting password").fill(PASSWORD);
  const [download] = await Promise.all([
    host.waitForEvent("download"),
    host.getByRole("button", { name: "Download", exact: true }).click(),
  ]);
  // saveAs works for remote browsers too (path() doesn't).
  const fs = await import("node:fs/promises");
  const os = await import("node:os");
  const path = `${os.tmpdir()}/tallies-${Date.now()}-${Math.random()}.json`;
  await download.saveAs(path);
  const tallies = JSON.parse(await fs.readFile(path, "utf8"));
  expect(tallies).toHaveLength(1);
  expect(tallies[0].round).toBe("Chair");
  expect(tallies[0].candidates).toEqual(["Anna", "Bo", "Cecilia"]);
  expect(tallies[0].score).toEqual([0, 1, 0]);
  expect(tallies[0].blank).toBe(1);
  expect(tallies[0].counts).toEqual({ eligible: 2, signed: 2, received: 2 });

  await host.getByRole("button", { name: "Yes, close meeting" }).click();
  await expect(host).toHaveURL(/\/create-meeting/);
  await expect(voter.getByText("Not in meeting")).toBeVisible({
    timeout: 15_000,
  });
});

test("the wrong password doesn't decrypt the tallies", async ({ browser }) => {
  const host = await createMeeting(browser);
  await startRound(host, "R", ["A", "B"]);
  await host.getByRole("button", { name: "A", exact: true }).click();
  await host.getByRole("button", { name: "Submit vote" }).click();
  await host.getByRole("button", { name: "Tally votes" }).click();
  await host.getByRole("button", { name: "Close meeting" }).click();
  await host.getByPlaceholder("Meeting password").fill("not the password");
  await host.getByRole("button", { name: "Download", exact: true }).click();
  await expect(
    host.getByText(/check that you entered the correct password/),
  ).toBeVisible({
    timeout: 30_000,
  });
});

test("a removed voter is told they're no longer in the meeting", async ({
  browser,
}) => {
  const host = await createMeeting(browser);
  const voter = await join(browser, await invite(host, "Bo"));
  const row = host.locator("div.px-5.py-3", { hasText: "Bo" });
  await row.getByRole("button", { name: "×" }).click();
  await expect(voter.getByText("Not in meeting")).toBeVisible({
    timeout: 15_000,
  });
});

test("agenda: upload, step through live, take attendance, download", async ({
  browser,
}) => {
  const host = await createMeeting(browser);
  const voter = await join(browser, await invite(host, "Anna"));

  // Upload the agenda as a file.
  await host.getByRole("button", { name: "Add agenda" }).click();
  await host.getByTestId("agenda-file").setInputFiles({
    name: "agenda.md",
    mimeType: "text/markdown",
    buffer: Buffer.from(
      "# Opening\nWelcome.\n\n# Election of chair\n## Nominations\n\n# Closing\n",
    ),
  });
  await expect(host.getByLabel("Agenda (Markdown)")).toHaveValue(/# Closing/);
  await host.getByRole("button", { name: "Save agenda" }).click();

  const current = (page: Page) => page.locator('li[aria-current="step"]');
  await expect(current(host)).toContainText("Opening");
  await expect(current(voter)).toContainText("Welcome.");

  // The voter's page follows the host forward and back by itself.
  await host.getByRole("button", { name: "Next →" }).click();
  await host.getByRole("button", { name: "Next →" }).click();
  await expect(current(voter)).toContainText("Nominations");
  await host.getByRole("button", { name: "← Previous" }).click();
  await expect(current(voter)).toContainText("Election of chair");

  await host.getByRole("button", { name: "Take attendance" }).click();
  await expect(
    host.getByText("Attendance recorded: 2 members at “Election of chair”."),
  ).toBeVisible();

  // Editing keeps the meeting on the same point.
  await host.getByRole("button", { name: "Edit", exact: true }).click();
  const editor = host.getByLabel("Agenda (Markdown)");
  await editor.fill(`# Minutes\n${await editor.inputValue()}`);
  await host.getByRole("button", { name: "Save agenda" }).click();
  await expect(current(voter)).toContainText("Election of chair");
  await expect(voter.locator("li")).toHaveCount(5);

  // The log downloads from the close-meeting panel.
  await host.getByRole("button", { name: "Close meeting" }).click();
  const [download] = await Promise.all([
    host.waitForEvent("download"),
    host.getByRole("button", { name: "Download attendance" }).click(),
  ]);
  expect(download.suggestedFilename()).toBe("attendance.json");
  const log = JSON.parse(
    await (await download.createReadStream())
      .toArray()
      .then((c) => Buffer.concat(c).toString()),
  );
  expect(log.records).toHaveLength(1);
  expect(log.records[0].point.title).toBe("Election of chair");
  expect(
    log.records[0].present.map((p: { name: string }) => p.name).sort(),
  ).toEqual(["Anna", "Host"]);
});
