import type { MeetingSpecsResponse } from "@/api/common/meetingSpecs";
import type { APIError } from "@/api/error";
import {
	EndVoteRound,
	type EndVoteRoundRequest,
	type TallyResponse,
} from "@/api/host/state";
import { matchResult } from "@/result";
import type React from "react";
import { useState } from "react";
import { VoteState } from "../host";

type TallyPageProps = {
	tally: TallyResponse | null;
	setCurrentState: React.Dispatch<React.SetStateAction<VoteState>>;
	setError: React.Dispatch<React.SetStateAction<APIError | null>>;
	specs: MeetingSpecsResponse | undefined;
	setTally: React.Dispatch<React.SetStateAction<TallyResponse | null>>;
	muid: string;
};

const TallyPage: React.FC<TallyPageProps> = ({
	tally,
	setCurrentState,
	setError,
	specs,
}) => {
	const [isLoading, setIsLoading] = useState(false);

	const handleBackToMeeting = async () => {
		setIsLoading(true);

		const result = await EndVoteRound({} as EndVoteRoundRequest);

		matchResult(result, {
			Ok: (_res) => {
				setCurrentState(VoteState.Creation);
				setIsLoading(false);
			},
			Err: (err) => {
				setError(err);
				setIsLoading(false);
			},
		});
	};

	// Process tally data for display
	const results = tally ? processResults(tally) : null;

	return (
		<div className="min-h-screen bg-gray-50">
			<div className="max-w-4xl mx-auto px-4 py-8">
				{/* Header */}
				<div className="text-center mb-8">
					<h1 className="text-3xl font-bold text-gray-900 mb-2">
						Vote Results
					</h1>
					<p className="text-gray-600">
						{specs?.title || "Meeting"} - Final Tally
					</p>
				</div>

				{/* Status Card */}
				<div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 mb-8">
					<div className="flex items-center gap-3">
						<div className="w-3 h-3 bg-green-500 rounded-full"></div>
						<div>
							<h2 className="text-lg font-semibold text-gray-900">
								Omröstning avslutad
							</h2>
							<p className="text-gray-600">Resultat beräknade och slutförda</p>
						</div>
					</div>
				</div>

				{/* Results */}
				{results ? (
					<div className="space-y-6">
						{/* Main Results */}
						<div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
							<h3 className="text-xl font-semibold text-gray-900 mb-6">
								Results
							</h3>

							{results.type === "candidates" ? (
								<CandidateResults results={results} />
							) : (
								<DichotomousResults results={results} />
							)}
						</div>

						{/* Summary Stats */}
						<div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
							<h3 className="text-xl font-semibold text-gray-900 mb-4">
								Summary
							</h3>
							<div className="grid grid-cols-1 md:grid-cols-3 gap-4">
								<div className="bg-blue-50 rounded-lg p-4 text-center">
									<div className="text-2xl font-bold text-blue-600">
										{results.totalVotes}
									</div>
									<div className="text-sm text-blue-800">Totala röster</div>
								</div>
								<div className="bg-gray-50 rounded-lg p-4 text-center">
									<div className="text-2xl font-bold text-gray-600">
										{results.blankVotes}
									</div>
									<div className="text-sm text-gray-800">Blanka röster</div>
								</div>
								<div className="bg-green-50 rounded-lg p-4 text-center">
									<div className="text-2xl font-bold text-green-600">
										{Math.round(
											((results.totalVotes - results.blankVotes) /
												results.totalVotes) *
												100,
										)}
										%
									</div>
									<div className="text-sm text-green-800">Deltagande</div>
								</div>
							</div>
						</div>
					</div>
				) : (
					<div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 text-center">
						<p className="text-gray-500">Inga resultat tillgängliga</p>
					</div>
				)}

				{/* Actions */}
				<div className="mt-8 flex justify-center">
					<button
						onClick={handleBackToMeeting}
						disabled={isLoading}
						className="px-6 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-blue-300 text-white font-medium rounded-md shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100 disabled:cursor-not-allowed"
					>
						{isLoading ? (
							<span className="flex items-center gap-2">
								<div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
								Avslutar omröstning...
							</span>
						) : (
							"Tillbaka till mötet"
						)}
					</button>
				</div>
			</div>
		</div>
	);
};

// Helper function to process tally results
function processResults(tally: TallyResponse) {
	const totalVotes =
		Object.values(tally.score).reduce((sum: number, count: any) => {
			return sum + (typeof count === "number" ? count : 0);
		}, 0) + tally.blank;

	// Check if it's a dichotomous vote (Yes/No)
	if ("Dichotomous" in tally.score) {
		const scores = tally.score["Dichotomous"] as number[];
		return {
			type: "dichotomous" as const,
			yes: scores[0] || 0,
			no: scores[1] || 0,
			totalVotes,
			blankVotes: tally.blank,
		};
	}

	// Otherwise it's candidate-based voting
	const candidates = Object.entries(tally.score)
		.map(([name, count]) => ({
			name,
			count: typeof count === "number" ? count : 0,
			percentage:
				totalVotes > 0
					? ((typeof count === "number" ? count : 0) / totalVotes) * 100
					: 0,
		}))
		.sort((a, b) => b.count - a.count);

	return {
		type: "candidates" as const,
		candidates,
		totalVotes,
		blankVotes: tally.blank,
	};
}

// Component for candidate-based results
const CandidateResults: React.FC<{ results: any }> = ({ results }) => {
	return (
		<div className="space-y-4">
			{results.candidates.map((candidate: any, index: number) => (
				<div key={candidate.name} className="flex items-center gap-4">
					<div className="flex-1">
						<div className="flex items-center justify-between mb-2">
							<span className="font-medium text-gray-900">
								{index === 0 && "🏆 "}
								{candidate.name}
							</span>
							<span className="text-sm text-gray-600">
								{candidate.count} votes ({Math.round(candidate.percentage)}%)
							</span>
						</div>
						<div className="w-full bg-gray-200 rounded-full h-3">
							<div
								className={`h-3 rounded-full transition-all duration-700 ${
									index === 0 ? "bg-green-500" : "bg-blue-500"
								}`}
								style={{ width: `${candidate.percentage}%` }}
							></div>
						</div>
					</div>
				</div>
			))}
		</div>
	);
};

// Component for dichotomous (Yes/No) results
const DichotomousResults: React.FC<{ results: any }> = ({ results }) => {
	const yesPercentage =
		results.totalVotes > 0 ? (results.yes / results.totalVotes) * 100 : 0;
	const noPercentage =
		results.totalVotes > 0 ? (results.no / results.totalVotes) * 100 : 0;
	const winner = results.yes > results.no ? "Ja" : "Nej";

	return (
		<div className="space-y-6">
			<div className="grid grid-cols-1 md:grid-cols-2 gap-4">
				<div
					className={`p-6 rounded-lg border-2 ${
						results.yes > results.no
							? "bg-green-50 border-green-200"
							: "bg-gray-50 border-gray-200"
					}`}
				>
					<div className="text-center">
						<div className="text-3xl font-bold text-green-600 mb-2">✓ YES</div>
						<div className="text-2xl font-semibold text-gray-900">
							{results.yes}
						</div>
						<div className="text-sm text-gray-600">
							{Math.round(yesPercentage)}% of votes
						</div>
					</div>
				</div>

				<div
					className={`p-6 rounded-lg border-2 ${
						results.no > results.yes
							? "bg-red-50 border-red-200"
							: "bg-gray-50 border-gray-200"
					}`}
				>
					<div className="text-center">
						<div className="text-3xl font-bold text-red-600 mb-2">✗ NO</div>
						<div className="text-2xl font-semibold text-gray-900">
							{results.no}
						</div>
						<div className="text-sm text-gray-600">
							{Math.round(noPercentage)}% of votes
						</div>
					</div>
				</div>
			</div>

			<div className="text-center p-4 bg-blue-50 rounded-lg">
				<div className="text-lg font-semibold text-blue-900">
					Resultat: <span className="text-2xl">{winner}</span> vinner
				</div>
			</div>
		</div>
	);
};

export default TallyPage;
