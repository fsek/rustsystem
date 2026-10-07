# Frontend testing

| Suite | Command | Needs |
|---|---|---|
| Unit and component tests | `pnpm test` | nothing |
| Whole meetings in Chromium, Firefox and WebKit | `pnpm test:e2e` | running services |

## Unit tests (`pnpm test`)

- `src/components/**` — every UI component.
- `src/voting/ballot.test.ts` — the ballot rules (the same as `rustsystem-server/src/ballot.rs`), the exact message format, signing with a real RSA blind-signature key, rejecting a signature from a key other than the server's, and submission retries (`AlreadyReceived` counts as success; no cookies are sent).
- `src/utils/cryptoGen.test.ts` — Argon2id key derivation against **test vectors shared with `decrypt-tally`**, fresh salts, and that only the public key leaves the browser.
- `src/utils/tallyDecrypt.test.ts` — decrypts a **file written by the Rust server code**, and rejects a wrong password or a tampered header.

The shared vectors and the fixture are what keep the browser, the server and the CLI agreeing on bytes. If you change the tally format or the KDF, regenerate them from the Rust side.

## Browser tests (`pnpm test:e2e`)

`e2e/meeting.spec.ts` runs a host and a voter in separate browser contexts through the production pages: create a meeting, invite, log in once, vote, reload (the page must remember the vote), vote blank as host, tally, decrypt all tallies with the password, close; plus a wrong password and a removed voter.

They need trustauth and the server running (serving `pnpm build`'s output), with rate limiting off:

```bash
pnpm build
RUSTSYSTEM_DISABLE_RATE_LIMIT=1 ../run_dev.sh &
pnpm test:e2e
```

WebKit doesn't run natively on every Linux distribution. Run it in Playwright's Docker image and set `PW_WEBKIT_WS` (see the main README's Testing section).
