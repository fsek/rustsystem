import { useEffect, useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { useNavigate } from '@tanstack/react-router';

export const Route = createFileRoute('/login')({
  validateSearch: (search) => {
    return {
      muid: search.muid ?? "",
      uuid: search.uuid ?? "",
    };
  },

  component: RouteComponent,
})

const LoginStatus = {
  Loading: 1,
  Success: 2,
  Failure: 3,
} as const;

type LoginStatus = (typeof LoginStatus)[keyof typeof LoginStatus];

function RouteComponent() {
  const [loginStatus, setLoginStatus] = useState<LoginStatus>(LoginStatus.Loading);
  const search = Route.useSearch();
  const navigate = useNavigate();

  const muid = search.muid;
  const uuid = search.uuid;

  useEffect(() => {
    login(muid, uuid).then((success) => {
      if (success) {
        setLoginStatus(LoginStatus.Success);
      } else {
        setLoginStatus(LoginStatus.Failure);
      }
    });
  }, []);


  if (loginStatus === LoginStatus.Loading) return <div>Checking...</div>;
  if (loginStatus === LoginStatus.Success) {
    navigate({ to: "/meeting", search: { muid: muid, uuid: uuid } });
    return <div>Logged in! Redirecting!</div>
  }
  if (loginStatus === LoginStatus.Failure) return <div>Login Failed!</div>;
}

async function login(muid: any, uuid: any): Promise<boolean> {
  const res = await fetch("api/login", {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ muid: muid, uuid: uuid })
  });

  const data = await res.json();
  const obj = JSON.parse(data);
  return obj["success"];
}
