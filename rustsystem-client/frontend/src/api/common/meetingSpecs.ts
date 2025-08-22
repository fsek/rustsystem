import { err, ok, type Result } from "@/result";

export type MeetingSpecsRequest = {};

export type MeetingSpecsResponse = {
  title: string;
  participants: number;
};

enum MeetingSpecsError {
  MUIDNotFound = "MUIDNotFound",
}

export async function MeetingSpecs(
  _req: MeetingSpecsRequest,
): Promise<Result<MeetingSpecsResponse, MeetingSpecsError>> {
  const res = await fetch("api/common/meeting-specs", {
    method: "GET",
    credentials: "include",
  });

  const obj = await res.json();
  if (res.ok) {
    return ok(obj as MeetingSpecsResponse);
  } else {
    return err(obj as MeetingSpecsError);
  }
}
