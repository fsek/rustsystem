import { err, ok, type Result } from "@/result";
import type { APIError } from "./error";

export type LoginRequest = {
  muid: any;
  uuid: any;
};

type LoginResponse = {};

export async function Login(
  req: LoginRequest,
): Promise<Result<LoginResponse, APIError>> {
  const res = await fetch("api/login", {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });

  if (res.ok) {
    return ok({} as LoginResponse);
  } else {
    const obj = await res.json();
    return err(obj as APIError);
  }
}

export const LoginStatus = {
  Loading: 1,
  Success: 2,
  Failure: 3,
} as const;

export type LoginStatus = (typeof LoginStatus)[keyof typeof LoginStatus];
