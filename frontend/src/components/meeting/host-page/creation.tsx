import type { MeetingSpecsResponse } from "@/api/common/meetingSpecs";
import type { APIError } from "@/api/error";
import { StartVote, type StartVoteRequest } from "@/api/host/state";
import { VoterList, type VoterListRequest } from "@/api/host/voterList";
import init, { BallotMetaData } from "@/pkg/rustsystem_client";
import { matchResult } from "@/result";
import type React from "react";
import { useState, useEffect } from "react";
import type { VoteState } from "../host";

type CreationPageProps = {
  specs: MeetingSpecsResponse | undefined;
  muid: string;
  setError: React.Dispatch<React.SetStateAction<APIError | null>>;
  currentState: VoteState;
  setCurrentState: React.Dispatch<React.SetStateAction<VoteState>>;
  setTally: React.Dispatch<React.SetStateAction<any>>;
};

const CreationPage: React.FC<CreationPageProps> = ({ specs, setError }) => {
  init();

  const [voteName, setVoteName] = useState("");
  const [candidates, setCandidates] = useState<string[]>(["", ""]);
  const [maxSelections, setMaxSelections] = useState(1);
  const [isLoading, setIsLoading] = useState(false);
  const [checkedInCount, setCheckedInCount] = useState(0);
  const [totalParticipants, setTotalParticipants] = useState(0);

  // Fetch voter list to get checked-in count
  useEffect(() => {
    const fetchVoterList = async () => {
      const result = await VoterList({} as VoterListRequest);
      matchResult(result, {
        Ok: (response) => {
          const checkedIn = response.voters.filter(
            (voter) => voter.loggedIn,
          ).length;
          setCheckedInCount(checkedIn);
          setTotalParticipants(response.voters.length);
        },
        Err: (err) => {
          console.error("Failed to fetch voter list:", err);
          // Fallback to specs participants if available
          setTotalParticipants(specs?.participants || 0);
        },
      });
    };

    fetchVoterList();
    // Refresh every 10 seconds to keep count updated
    const interval = setInterval(fetchVoterList, 10000);
    return () => clearInterval(interval);
  }, [specs]);

  const handleAddCandidate = () => {
    setCandidates([...candidates, ""]);
  };

  const handleRemoveCandidate = (index: number) => {
    if (candidates.length > 2) {
      setCandidates(candidates.filter((_, i) => i !== index));
    }
  };

  const handleCandidateChange = (index: number, value: string) => {
    const newCandidates = [...candidates];
    newCandidates[index] = value;
    setCandidates(newCandidates);
  };

  const handleStartVote = async () => {
    if (!voteName.trim()) {
      setError({
        code: "ValidationError",
        message: "Omröstningsnamn krävs",
        httpStatus: 400,
        timestamp: new Date().toISOString(),
        endpoint: { method: "POST", path: "/api/host/start-vote" },
      });
      return;
    }

    const validCandidates = candidates.filter((c) => c.trim() !== "");
    if (validCandidates.length < 2) {
      setError({
        code: "ValidationError",
        message: "Minst 2 alternativ krävs",
        httpStatus: 400,
        timestamp: new Date().toISOString(),
        endpoint: { method: "POST", path: "/api/host/start-vote" },
      });
      return;
    }

    setIsLoading(true);

    const result = await StartVote({
      name: voteName.trim(),
      metadata: new BallotMetaData(
        validCandidates,
        0,
        Math.min(maxSelections, validCandidates.length),
      ),
    } as StartVoteRequest);

    matchResult(result, {
      Ok: (_res) => {
        setIsLoading(false);
        // State will be updated automatically via WebSocket
      },
      Err: (err) => {
        setError(err);
        setIsLoading(false);
      },
    });
  };

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="max-w-4xl mx-auto px-4 py-8">
        {/* Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">
            {specs?.title || "Möte"}
          </h1>
          <div className="flex items-center justify-center gap-4 text-sm text-gray-600">
            <span className="flex items-center gap-1">
              👥 {totalParticipants} deltagare
            </span>
            <span className="flex items-center gap-1">
              ✅ {checkedInCount} incheckade
            </span>
            <span className="flex items-center gap-1">📊 Redo att rösta</span>
          </div>
        </div>

        {/* Status Card */}
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 mb-8">
          <div className="flex items-center gap-3">
            <div className="w-3 h-3 bg-blue-500 rounded-full"></div>
            <div>
              <h2 className="text-lg font-semibold text-gray-900">
                Skapandefas
              </h2>
              <p className="text-gray-600">
                Ställ in din omröstning och starta när du är redo
              </p>
            </div>
          </div>
        </div>

        {/* Vote Setup Form */}
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
          <h3 className="text-xl font-semibold text-gray-900 mb-6">
            Skapa ny omröstning
          </h3>

          {/* Vote Name */}
          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Omröstningsnamn *
            </label>
            <input
              type="text"
              value={voteName}
              onChange={(e) => setVoteName(e.target.value)}
              placeholder="t.ex. 'Styrelsemedlemsval', 'Budgetförslag'"
              className="w-full p-3 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
          </div>

          {/* Max Selections */}
          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Maximalt antal val
            </label>
            <div className="flex items-center gap-4">
              <input
                type="number"
                min="0"
                max={Math.max(1, candidates.filter((c) => c.trim()).length)}
                value={maxSelections}
                onChange={(e) =>
                  setMaxSelections(
                    Math.max(0, Number.parseInt(e.target.value) || 0),
                  )
                }
                className="w-24 p-3 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
              <span className="text-sm text-gray-600">
                {maxSelections === 0
                  ? "Endast blanka röster"
                  : maxSelections === 1
                    ? "Ett val"
                    : `Välj upp till ${maxSelections} alternativ`}
              </span>
            </div>
            <p className="text-xs text-gray-500 mt-1">
              Sätt till 0 för att endast tillåta blanka röster, eller välj hur
              många alternativ väljarna kan välja
            </p>
          </div>

          {/* Candidates */}
          <div className="mb-8">
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Alternativ/Kandidater *
            </label>
            <div className="space-y-3">
              {candidates.map((candidate, index) => (
                <div key={index} className="flex gap-3">
                  <input
                    type="text"
                    value={candidate}
                    onChange={(e) =>
                      handleCandidateChange(index, e.target.value)
                    }
                    placeholder={`Alternativ ${index + 1}`}
                    className="flex-1 p-3 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  />
                  {candidates.length > 2 && (
                    <button
                      type="button"
                      onClick={() => handleRemoveCandidate(index)}
                      className="px-3 py-2 text-red-600 hover:text-red-800 hover:bg-red-50 rounded-md transition-colors"
                    >
                      ✕
                    </button>
                  )}
                </div>
              ))}
            </div>

            <button
              type="button"
              onClick={handleAddCandidate}
              className="mt-3 text-blue-600 hover:text-blue-800 text-sm font-medium"
            >
              + Lägg till alternativ
            </button>
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-3">
            <button
              type="button"
              onClick={handleStartVote}
              disabled={isLoading}
              className="px-6 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-blue-300 text-white font-medium rounded-md shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100 disabled:cursor-not-allowed"
            >
              {isLoading ? (
                <span className="flex items-center gap-2">
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                  Startar omröstning...
                </span>
              ) : (
                "Starta omröstning"
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default CreationPage;
