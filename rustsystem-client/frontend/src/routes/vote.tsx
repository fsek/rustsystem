import { createFileRoute, Link } from '@tanstack/react-router'
import { useEffect } from "react";
import init, { test_register, send_vote } from "@/pkg/rustsystem_client.js";

export const Route = createFileRoute("/vote")({
  component: RouteComponent,
});

function RouteComponent() {

  useEffect(() => {
    init().then(() => {
      test_register().then((result) => {
        console.log(result.signature());
        console.log(result.proof());

        send_vote(result).then((response) => {
          console.log(response);
        });
      });
    });
  }, []);

  return <div>Hello from /vote! <Link to="/">Back home</Link></div>
}
