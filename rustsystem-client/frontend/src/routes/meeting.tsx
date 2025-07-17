import { createFileRoute } from '@tanstack/react-router'
import { Auth } from '../auth.ts'

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

  Auth(search.muid).then((res) => {
    if (res) {
      console.log("Successfully logged in");
    } else {
      console.log("Could not log in");
    }
  });
  
  return <div>Hello "/meeting"!</div>
}
