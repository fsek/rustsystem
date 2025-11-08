import { useEffect, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { useNavigate } from "@tanstack/react-router";
import { Login, LoginStatus } from "@/api/login";
import { matchResult } from "@/result";
import type { APIError } from "@/api/error";
import ErrorHandler from "@/components/error";

type SearchParams = {
  muuid: string;
  uuuid: string;
};

export const Route = createFileRoute("/login")({
  validateSearch: (search): SearchParams => {
    return {
      muuid: (search.muuid as string) ?? "",
      uuuid: (search.uuuid as string) ?? "",
    };
  },

  component: RouteComponent,
});

function RouteComponent() {
  const [loginStatus, setLoginStatus] = useState<LoginStatus>(
    LoginStatus.Loading,
  );
  const [error, setError] = useState<APIError | null>(null);
  const search = Route.useSearch();
  const navigate = useNavigate();

  const muuid = search.muuid;
  const uuuid = search.uuuid;

  useEffect(() => {
    Login({ muuid, uuuid }).then((result) => {
      matchResult(result, {
        Ok: (_res) => {
          setLoginStatus(LoginStatus.Success);
        },
        Err: (err) => {
          setError(err);
        },
      });
    });
  }, []);

  if (error) {
    return <ErrorHandler error={error} />;
  }
  if (loginStatus === LoginStatus.Loading) return <div>Checking...</div>;
  if (loginStatus === LoginStatus.Success) {
    navigate({ to: "/meeting", search: { muuid, uuuid } });
    return <div>Logged in! Redirecting!</div>;
  }
}
