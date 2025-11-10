import { type Result, err, ok } from "@/result";
import type { APIError } from "../error";

export type MeetingSpecsRequest = {};

export type MeetingSpecsResponse = {
  title: string;
  participants: number;
  agenda: string;
};

export async function MeetingSpecs(
  _req: MeetingSpecsRequest,
): Promise<Result<MeetingSpecsResponse, APIError>> {
  const res = await fetch("/api/common/meeting-specs", {
    method: "GET",
    credentials: "include",
  });

  const obj = await res.json();
  if (res.ok) {
    return ok(obj as MeetingSpecsResponse);
  } else {
    return err(obj as APIError);
  }
}

export function meetingSpecsWatch(): EventSource {
  return new EventSource("/api/common/meeting-specs-watch");
}

export type UpdateAgendaRequest = {
  agenda: string;
};

export async function updateAgenda(
  req: UpdateAgendaRequest,
): Promise<Result<{}, APIError>> {
  const res = await fetch("/api/common/update-agenda", {
    method: "POST",
    credentials: "include",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(req),
  });

  if (res.ok) {
    return ok({});
  } else {
    const obj = await res.json();
    return err(obj as APIError);
  }
}
