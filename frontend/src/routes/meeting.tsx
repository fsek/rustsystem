import { Auth, type AuthMeetingRequest, AuthStatus } from "@/api/auth";
import {
	MeetingSpecs,
	type MeetingSpecsRequest,
	type UpdateAgendaRequest,
	updateAgenda,
} from "@/api/common/meetingSpecs";
import type { APIError } from "@/api/error";
import ErrorHandler from "@/components/error";
import Unauthorized from "@/components/error-pages/unauthorized.tsx";
import HostPage from "@/components/meeting/host";
import FloatingControls from "@/components/meeting/host-widget/floating-controls";
import VoterPage from "@/components/meeting/voter";
import { matchResult } from "@/result";
import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useRef, useState } from "react";
import "@/colors.css";

type SearchParams = {
	muuid: string;
	uuuid: string;
};

export const Route = createFileRoute("/meeting")({
	validateSearch: (search): SearchParams => {
		return {
			muuid: (search.muuid as string) ?? "",
			uuuid: (search.uuuid as string) ?? "",
		};
	},

	component: RouteComponent,
});

function RouteComponent() {
	const [authStatus, setAuthStatus] = useState<AuthStatus>(AuthStatus.Loading);
	const [error, setError] = useState<APIError | null>(null);
	const [agenda, setAgenda] = useState<string>("");
	const [isUpdatingAgenda, setIsUpdatingAgenda] = useState(false);
	const debounceTimerRef = useRef<number | null>(null);
	const search = Route.useSearch();
	const muuid = search.muuid;
	const uuid = search.uuuid;

	useEffect(() => {
		Auth({ muuid } satisfies AuthMeetingRequest).then((result) => {
			matchResult(result, {
				Ok: (res) => {
					console.log("Auth response:", res);
					console.log("Is host:", res.is_host);
					if (res.is_host) {
						console.log("Setting auth status to VerifiedHost");
						setAuthStatus(AuthStatus.VerifiedHost);
					} else {
						console.log("Setting auth status to VerifiedNonHost");
						setAuthStatus(AuthStatus.VerifiedNonHost);
					}
				},
				Err: (err) => {
					console.error("Auth error:", err);
					setError(err);
				},
			});
		});

		// Fetch meeting specs to get agenda
		MeetingSpecs({} as MeetingSpecsRequest).then((result) => {
			matchResult(result, {
				Ok: (specsData) => {
					setAgenda(specsData.agenda);
				},
				Err: (err) => {
					setError(err);
				},
			});
		});
	}, []);

	// Cleanup timer on unmount
	useEffect(() => {
		return () => {
			if (debounceTimerRef.current) {
				clearTimeout(debounceTimerRef.current);
			}
		};
	}, []);

	const debouncedSaveAgenda = useCallback(async (agendaText: string) => {
		setIsUpdatingAgenda(true);
		const result = await updateAgenda({
			agenda: agendaText,
		} as UpdateAgendaRequest);
		matchResult(result, {
			Ok: () => {
				// Success - agenda saved
			},
			Err: (err) => {
				setError(err);
			},
		});
		setIsUpdatingAgenda(false);
	}, []);

	const handleAgendaChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
		const newAgenda = e.target.value;
		setAgenda(newAgenda);

		// Clear existing timer
		if (debounceTimerRef.current) {
			clearTimeout(debounceTimerRef.current);
		}

		// Set new timer
		debounceTimerRef.current = setTimeout(() => {
			debouncedSaveAgenda(newAgenda);
		}, 500);
	};

	if (error) {
		return <ErrorHandler error={error} />;
	}

	let rightPaneContent = null;

	if (authStatus === AuthStatus.Loading) {
		rightPaneContent = (
			<div className="flex items-center justify-center h-full">
				<div className="text-lg text-gray-600">Autentiserar...</div>
			</div>
		);
	} else if (authStatus === AuthStatus.VerifiedHost) {
		rightPaneContent = <HostPage muid={muuid} />;
	} else if (authStatus === AuthStatus.VerifiedNonHost) {
		rightPaneContent = <VoterPage muid={muuid} uuid={uuid} />;
	} else if (authStatus === AuthStatus.Denied) {
		rightPaneContent = <Unauthorized />;
	}

	// Host view - split panes with agenda
	if (authStatus === AuthStatus.VerifiedHost) {
		return (
			<div className="h-screen bg-[var(--color-background)] flex">
				{/* Left Pane - Agenda */}
				<div className="w-1/2 border-r border-gray-200 flex flex-col">
					<div className="p-6 border-b border-gray-200 bg-white">
						<h2 className="text-xl font-semibold text-[var(--color-contours)] mb-2">
							Mötesagenda
						</h2>
						<p className="text-sm text-gray-600">
							Använd detta utrymme för att spåra dagordningspunkter och
							anteckningar
						</p>
					</div>
					<div className="flex-1 p-6 bg-white">
						<textarea
							value={agenda}
							onChange={handleAgendaChange}
							placeholder="Lägg till dagordningspunkter, anteckningar och diskussionspunkter här..."
							className="w-full h-full resize-none border-0 focus:outline-none focus:ring-0 text-gray-700 placeholder-gray-400 text-base leading-relaxed"
							style={{
								fontFamily:
									'ui-monospace, "SF Mono", Monaco, "Cascadia Code", "Roboto Mono", Consolas, "Courier New", monospace',
							}}
						/>
						{isUpdatingAgenda && (
							<div className="absolute bottom-2 right-2 text-xs text-gray-400">
								Sparar...
							</div>
						)}
					</div>
				</div>

				{/* Right Pane - Host Content */}
				<div className="w-1/2 flex flex-col overflow-hidden">
					<div className="flex-1 overflow-y-auto">{rightPaneContent}</div>
				</div>

				{/* Floating Controls */}
				<FloatingControls muid={muuid} setError={setError} />
			</div>
		);
	}

	// Voter/other views - normal full-width layout
	return (
		<div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
			{rightPaneContent}
		</div>
	);
}
