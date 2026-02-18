# Frontend Testing

## Quick reference

| Suite | Command | Needs server? | Needs Docker? |
|---|---|---|---|
| Unit & component tests | `pnpm test` | No (skips server tests) | No |
| Unit & component + server integration | `pnpm test` | Yes | No |
| Playwright e2e (all browsers) | `pnpm test:e2e:docker` | Yes | Yes |
| Playwright e2e (single browser) | `pnpm test:e2e:docker --project=chromium` | Yes | Yes |

Start the Rust backend for server-dependent suites:
```bash
API_ENDPOINT=http://localhost:3000 cargo run --bin rustsystem-server
```

---

## Signatures

**Run:** `pnpm test`

Two layers of tests live under `src/signatures/`:

### Unit tests — `src/signatures/signatures.test.ts`

Pure in-process tests with no network calls. They exercise the BBS+ cryptographic primitives implemented in `signatures.ts`:

- **`uuidToBytes`** — correct byte length (16), determinism, byte-for-byte correctness
- **`createBlindGenerators`** — correct count, 48-byte compressed G1 points, determinism across calls, distinct points, stability when requesting more generators
- **`hashToScalar` / `messageToScalar`** — returns a bigint in the BLS12-381 scalar field, deterministic, different inputs produce different scalars, uses a different DST from a plain hash
- **`commit`** — returns `commitmentJson` and `blindFactor`; blind factor is 32 bytes; commitment JSON has the `BBSplus` wrapper with a 48-byte hex commitment, 32-byte `s_cap`, one 32-byte `m_cap`, 32-byte `challenge`; same token produces different commitments each call (random blind factor); serialises cleanly to JSON
- **`generateToken`** — 256-byte token, 32-byte blind factor, valid commitment JSON shape, valid context shape, two calls produce different tokens
- **`buildBallot`** — output contains `metadata`, `validation`, `choice`, and `_padding`; serialised JSON is at least 1 024 bytes; `validation.proof` is 32 numbers; `validation.token` is 256 numbers; `null` and `[0]` choices are preserved

### Server integration tests — `src/signatures/e2e-tests/`

Vitest tests running in the `node` environment that make real HTTP calls to the Rust backend. Each test creates an isolated meeting so tests are independent and can run in any order. Tests are skipped automatically (via `describe.skipIf`) when the server is not reachable.

**`meeting.test.ts`** — meeting creation and vote-round state machine:

- **`createMeeting`** — returns a valid `muuid` and `uuuid`; sets a session cookie so subsequent host calls succeed (401 otherwise)
- **`startVoteRound`** — succeeds from Idle state; returns 409 if a round is already active; returns 409 after tally (Tally state); returns 409 for duplicate candidate names; returns 409 when `max_choices` exceeds the candidate count
- **`tally`** — succeeds in Voting state, returns a score map and blank count initialised to zero; returns 410 from Idle state; returns 410 when called a second time (Tally state)
- **`getTally`** — returns the stored tally matching the `tally` response; returns 410 before a tally has been computed; returns 410 after `end-vote-round` resets state
- **`endVoteRound`** — succeeds from Voting state; succeeds from Tally state (normal cleanup path); idempotent — also succeeds from Idle state
- **Full cycle** — Idle → start → register → submit → tally → end → start again (verifies state resets so a second round can begin)

**`registration.test.ts`** — `POST /api/voter/register`:

- **Happy path** — returns a BBS+ blind signature and the round metadata; locally generated token is non-empty
- **State constraints** — returns 410 before a vote round has started; returns 410 after the round has been tallied; returns 410 after `end-vote-round`
- **Duplicate prevention** — returns 409 on a second registration by the same user (one-person-one-vote)
- **Authentication** — returns 401 with no session cookie
- **Malformed requests** — returns 4xx when the `commitment` field is missing; returns 4xx when `commitment` is not a valid object; documents that `context.voter_id` is not validated against the JWT (current intentional behaviour)

**`submission.test.ts`** — `POST /api/voter/submit`:

- **Happy path** — accepts a single-candidate vote and confirms it in the tally; accepts a blank (`null`) vote recorded in the blank counter; accepts multi-choice votes when `max_choices > 1`
- **State constraints** — returns 410 when no round is active; returns 410 after the round has been tallied
- **Signature attacks** — returns 401 when a fresh token is paired with an old signature (token–signature binding); returns 401 when a single byte of the scalar `e` is flipped (BBS+ verification fails); returns 422 when the signature is structurally malformed (Axum deserialisation failure, not a crypto failure); returns 409 on replay — re-submitting the same signature after a successful vote
- **Ballot validation** — returns 409 when ballot metadata does not match the active round; returns 409 when the number of choices exceeds `max_choices`
- **Authentication** — returns 401 with no session cookie

---

## E2E

**Run:** `pnpm test:e2e:docker [playwright-args]`

Playwright tests in `e2e/voting.spec.ts` run inside an Ubuntu Docker container (`mcr.microsoft.com/playwright`) so all five browser engines have their expected system libraries. Tests hit the Vite dev server (started automatically by Playwright's `webServer` option inside Docker) and the Rust backend on the host via `--network=host`.

**Prerequisites:**
1. Docker running
2. Rust backend running on `localhost:3000`

```bash
API_ENDPOINT=http://localhost:3000 cargo run --bin rustsystem-server
pnpm test:e2e:docker
```

The suite runs across five browser/device profiles: Chromium, Firefox, WebKit (Safari engine), mobile Chrome (Pixel 7), and mobile Safari (iPhone 14).

**`full vote cycle`** — the canonical happy path across all browsers:
- Candidate vote (choice index 0) — verifies the token is cleared from `localStorage` after a successful submission
- Blank vote (empty choice) — verifies the server's abstain path

**`localStorage persistence`** — verifies token storage behaviour:
- Token survives a hard page reload — after registering, reloading the page shows the "restored from storage" banner and keeps the Submit button enabled so the voter can still submit
- Token is cleared after successful submission — after voting, reload shows no banner and Submit is disabled
- Clear token button — clicking the button removes the token from `localStorage` and disables Submit

**`cookie handling`** — verifies HttpOnly JWT cookie behaviour in real browser cookie jars:
- `access_token` cookie is set after `createMeeting` and automatically sent on subsequent requests (Start Vote succeeds, proving the cookie jar is wired up)
- Cookie persists across a same-origin navigation (navigate away to `/` and back to `/signature-dev`)

**`UI state machine`** — verifies that the step gates work correctly:
- Start Vote and Register buttons are disabled until their prerequisites are met
- Submit Vote button is disabled until registration is complete
- End Vote Round resets the active-round state (Register becomes disabled again)

**`mobile viewport`** — smoke test for layout on small screens:
- Full vote cycle completes on a Pixel 7 / iPhone 14 viewport (narrow layout, touch events, mobile UA)

---

## Components

**Run:** `pnpm test`

Vitest + jsdom + Testing Library tests for each UI component in `src/components/`. Every component accepts a `size` (`"s"` | `"sm"` | `"m"` | `"ml"` | `"l"` | `"xl"`) and a `color` (`"primary"` | `"secondary"` | `"accent"`). Tests verify rendering at each size and color combination does not throw, along with component-specific behaviour:

- **`Alert`** — renders children; has `role="alert"`; applies a left-border style; renders the info icon (ℹ); passes `className` through
- **`Badge`** — renders children; renders the correct HTML element; applies styles; renders across all sizes and colors
- **`Button`** — renders children as a `<button>`; applies filled style by default; applies outline style with `variant="outline"`; forwards standard HTML button props (`disabled`, `className`, `data-testid`, etc.); renders across all sizes and colors
- **`Card`** — renders the `title` prop; renders children; applies styles; renders across all sizes and colors
- **`Input`** — renders an `<input>` element; passes value and onChange; forwards HTML input props; renders across all sizes and colors
- **`Spinner`** — renders a visible spinner element; renders across all sizes and colors
- **`VoteOption`** — renders the option label; renders a checkbox; reflects checked state; calls onChange on interaction
- **`VoteSection`** — renders a section with multiple `VoteOption` children; tracks selected options; respects `maxChoices`
