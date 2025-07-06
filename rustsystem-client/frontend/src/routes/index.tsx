import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
	component: App,
});

function App() {
	function createMeeting() {
		fetch("create-meeting", {
    	method: "POST",
    	headers: { "Content-Type": "application/json" },
    	body: JSON.stringify({ name: "Test Meeting" })
  	}).then((res) => {
  		console.log(res);
			res.json().then((url) => {
				console.log(url);
				window.location.href = url;
			});
  	});

	}
	
	return <button onClick={createMeeting}>Create Meeting</button>
}
