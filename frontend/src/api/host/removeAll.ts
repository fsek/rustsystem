import { type Result, err, ok } from "@/result";
import type { APIError } from "../error";

export type RemoveAllRequest = {};

export async function removeAll(
  _req: RemoveAllRequest,
): Promise<Result<{}, APIError>> {
  const res = await fetch("/api/host/remove-all", {
    method: "DELETE",
    credentials: "include",
  });

  if (res.ok) {
    return ok({});
  } else {
    const obj = await res.json();
    return err(obj as APIError);
  }
}
