export function startVoteWait(): EventSource {
  return new EventSource("/api/voter/vote-watch");
}
