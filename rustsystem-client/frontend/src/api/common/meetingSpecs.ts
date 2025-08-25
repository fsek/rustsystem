import { err, ok, type Result } from "@/result";
import type { APIError } from "../error";

export type MeetingSpecsRequest = {};

export type MeetingSpecsResponse = {
  title: string;
  participants: number;
};

export async function MeetingSpecs(
  _req: MeetingSpecsRequest,
): Promise<Result<MeetingSpecsResponse, APIError>> {
  const res = await fetch("api/common/meeting-specs", {
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
