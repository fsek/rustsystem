export function startInviteWait(): EventSource {
	return new EventSource("/api/host/invite-watch");
}
