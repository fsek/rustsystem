import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
	component: App,
});

function App() {
	function createMeeting() {
		fetch("create-meeting", {
    	method: "POST",
    	credentials: "include",
    	headers: { "Content-Type": "application/json" },
    	body: JSON.stringify({ user_name: "Test User", meeting_name: "Test Meeting" })
  	}).then((res) => {
  		console.log(res.json());
  		fetch("/protected", {
  				method: "GET",
  				credentials: "include",
  			}).then((res) => {
  					console.log(res.json());
  					console.log(res);
  				});
  	});
	}
	
	return <button onClick={createMeeting}>Create Meeting</button>
}
