import { useEffect, useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { useNavigate } from '@tanstack/react-router';
import { Login, LoginStatus, type LoginRequest } from '@/api/login';
import { matchResult } from '@/result';

export const Route = createFileRoute('/login')({
  validateSearch: (search) => {
    return {
      muid: search.muid ?? "",
      uuid: search.uuid ?? "",
    };
  },

  component: RouteComponent,
})


function RouteComponent() {
  const [loginStatus, setLoginStatus] = useState<LoginStatus>(LoginStatus.Loading);
  const search = Route.useSearch();
  const navigate = useNavigate();

  const muid = search.muid;
  const uuid = search.uuid;

  useEffect(() => {
    Login({ muid, uuid } as LoginRequest).then((result) => {
      matchResult(result, {
        Ok: (_res) => {
          setLoginStatus(LoginStatus.Success);
        },
        Err: (err) => {
          console.error(err);
          setLoginStatus(LoginStatus.Failure);
        }
      })
    });
  }, []);


  if (loginStatus === LoginStatus.Loading) return <div>Checking...</div>;
  if (loginStatus === LoginStatus.Success) {
    navigate({ to: "/meeting", search: { muid: muid, uuid: uuid } });
    return <div>Logged in! Redirecting!</div>
  }
  if (loginStatus === LoginStatus.Failure) return <div>Login Failed!</div>;
}
