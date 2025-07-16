import { createFileRoute, useNavigate } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
	component: App,
});

function App() {
	const navigate = useNavigate();
	
	function createMeeting() {
		fetch("api/create-meeting", {
    	method: "POST",
    	credentials: "include",
    	headers: { "Content-Type": "application/json" },
    	body: JSON.stringify({ title: "Test Meeting" })
  	}).then((res) => {
  			res.json().then((data) => {
  				navigate({ to: "/meeting", search: { muid: data.muid }});
  			});
  	});
	}
	
	return <button onClick={createMeeting}>Create Meeting</button>
}
