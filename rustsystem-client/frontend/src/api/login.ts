import { err, ok, type Result } from "@/result";

export type LoginRequest = {
  muid: any;
  uuid: any;
};

type LoginResponse = {};

enum LoginError {
  InvalidUUID = "InvalidUUID",
  InvalidMUID = "InvalidMUID",

  UUIDAlreadyClaimed = "UUIDAlreadyClaimed",
  UUIDNotFound = "UUIDNotFound",
  MUIDNotFound = "MUIDNotFound",
}

export async function Login(
  req: LoginRequest,
): Promise<Result<LoginResponse, LoginError>> {
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
    return err(obj as LoginError);
  }
}

export const LoginStatus = {
  Loading: 1,
  Success: 2,
  Failure: 3,
} as const;

export type LoginStatus = (typeof LoginStatus)[keyof typeof LoginStatus];
