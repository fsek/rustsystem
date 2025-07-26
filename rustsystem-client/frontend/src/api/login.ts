export type LoginRequest = {
  muid: any;
  uuid: any;
};

type LoginResponse = {
  success: boolean;
};

export async function Login(req: LoginRequest): Promise<LoginResponse> {
  const res = await fetch("api/login", {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });

  const data = await res.json();
  const obj = JSON.parse(data);
  return obj as LoginResponse;
}
