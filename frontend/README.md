# Rustsystem frontend

React 19 + TanStack Router. See the [main README](../README.md) for how the system works and how to run it, and [TESTING.md](TESTING.md) for the test suites.

| Path | What it is |
|---|---|
| `src/routes/` | Pages (file-based routing): `/create-meeting`, `/login`, `/meeting`, `/admin`, and the guide pages. `/dev/preview` shows every UI component (only when built with `DEV=true`). |
| `src/components/` | UI components, each with its tests. |
| `src/api/` | Talking to the backends: `client.ts` (fetch + errors), `meeting.ts`, `host.ts`. |
| `src/voting/ballot.ts` | Casting a vote: build, check, blind, sign, submit. |
| `src/utils/` | The meeting's tally key (`cryptoGen.ts`), decrypting tally files (`tallyDecrypt.ts`), result exports. |
| `e2e/` | Playwright tests of whole meetings in real browsers. |

```bash
pnpm install
pnpm dev     # http://localhost:3000, with both backends running (../run_dev.sh)
pnpm build   # production build into dist/, served by rustsystem-server
```

The only build-time setting is `DEV` (enables the `/dev` pages). Where trustauth is comes from the server at runtime (`GET /api/config`), so the same build works everywhere. Open the app at `localhost`, not `127.0.0.1`: trustauth's cookie only works when both are on the same host name.
