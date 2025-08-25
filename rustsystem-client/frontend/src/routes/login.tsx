import { useEffect, useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { useNavigate } from '@tanstack/react-router';
import { Login, LoginStatus, type LoginRequest } from '@/api/login';
import { matchResult } from '@/result';
import type { APIError } from '@/api/error';
import ErrorHandler from '@/components/error';

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
  const [error, setError] = useState<APIError | null>(null);
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
          setError(err);
        }
      })
    });
  }, []);

  if (error) {
    return <ErrorHandler error={error} />
  }
  if (loginStatus === LoginStatus.Loading) return <div>Checking...</div>;
  if (loginStatus === LoginStatus.Success) {
    navigate({ to: "/meeting", search: { muid: muid, uuid: uuid } });
    return <div>Logged in! Redirecting!</div>
  }
}
