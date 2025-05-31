import { createFileRoute, Link } from '@tanstack/react-router'

export const Route = createFileRoute('/invite')({
  component: RouteComponent,
})

function RouteComponent() {
  return <div>Hello "/invite"! <Link to="/">Back home</Link></div>
}
