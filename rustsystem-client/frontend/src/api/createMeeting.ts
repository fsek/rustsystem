import { err, ok, type Result } from "@/result";
import type { APIError } from "./error";

export type CreateMeetingRequest = {
  title: string;
  host_name: string;
};

type CreateMeetingResponse = {
  muuid: string;
  uuuid: string;
};

export async function CreateMeeting(
  req: CreateMeetingRequest,
): Promise<Result<CreateMeetingResponse, APIError>> {
  const res = await fetch("api/create-meeting", {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });

  const obj = await res.json();
  if (res.ok) {
    return ok(obj as CreateMeetingResponse);
  } else {
    return err(obj as APIError);
  }
}
