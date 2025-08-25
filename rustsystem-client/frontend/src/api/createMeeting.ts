import { err, ok, type Result } from "@/result";
import type { APIError } from "./error";

export type CreateMeetingRequest = {
  title: string;
};

type CreateMeetingResponse = {
  muid: any;
  uuid: any;
};

export async function CreateMeeting(
  req: CreateMeetingRequest,
): Promise<Result<CreateMeetingResponse, APIError>> {
  console.log("got title", req.title);
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
