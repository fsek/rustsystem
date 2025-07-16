import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/meeting')({
  validateSearch: (search) => {
    return {
      muid: search.muid ?? "",
    };
  },

  component: RouteComponent,
})

function RouteComponent() {
  const search = Route.useSearch();

  fetch("api/auth-meeting", {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ muid: search.muid })
  }).then((res) => {
      res.json().then((data) => {
        console.log(data);
      })
    });
  
  return <div>Hello "/meeting"!</div>
}
