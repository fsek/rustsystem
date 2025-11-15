import type { MeetingSpecsResponse } from "@/api/common/meetingSpecs";
import type { APIError } from "@/api/error";
import { Tally, type TallyRequest, type TallyResponse } from "@/api/host/state";
import {
  getVoteProgress,
  voteProgressWatch,
  type VoteProgressResponse,
} from "@/api/common/voteProgress";
import { matchResult } from "@/result";
import type React from "react";
import { useEffect, useState } from "react";
import { VoteState } from "../host";

type VotingPageProps = {
  specs: MeetingSpecsResponse | undefined;
  setTally: React.Dispatch<React.SetStateAction<TallyResponse | null>>;
  setCurrentState: React.Dispatch<React.SetStateAction<VoteState>>;
  setError: React.Dispatch<React.SetStateAction<APIError | null>>;
  muid: string;
};

const VotingPage: React.FC<VotingPageProps> = ({
  specs,
  setTally,
  setCurrentState,
  setError,
}) => {
  const [isLoading, setIsLoading] = useState(false);
  const [voteProgress, setVoteProgress] = useState<VoteProgressResponse | null>(
    null,
  );

  // Define fetchVoteProgress function outside useEffect so it can be reused
  const fetchVoteProgress = async () => {
    const result = await getVoteProgress({});
    matchResult(result, {
      Ok: (response) => {
        setVoteProgress(response);
      },
      Err: (err) => {
        console.error("Failed to fetch vote progress:", err);
      },
    });
  };

  // Fetch vote progress and set up real-time updates
  useEffect(() => {
    fetchVoteProgress();

    // Set up Server-Sent Events for real-time updates
    const eventSource = voteProgressWatch();

    eventSource.onmessage = (event) => {
      // Event data is just the event name "VoteProgressUpdated"
      // We need to fetch the actual progress data
      if (event.data === "VoteProgressUpdated") {
        fetchVoteProgress();
      }
    };

    eventSource.onerror = () => {
      console.error("Vote progress SSE connection error");
      // Fallback to polling on error
      setTimeout(fetchVoteProgress, 2000);
    };

    return () => {
      eventSource.close();
    };
  }, []);

  const handleTally = async () => {
    setIsLoading(true);

    const result = await Tally({} as TallyRequest);

    matchResult(result, {
      Ok: (res) => {
        setTally(res);
        setCurrentState(VoteState.Tally);
        setIsLoading(false);
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
            {specs?.title || "Meeting"}
          </h1>
          {voteProgress && (
            <div className="flex items-center justify-center gap-4 text-sm text-gray-600">
              <span className="flex items-center gap-1">
                👥 {voteProgress.totalParticipants} deltagare
              </span>
              <span className="flex items-center gap-1">
                🗳️ {voteProgress.totalVotesCast} röster inkomna
              </span>
            </div>
          )}
        </div>

        {/* Status Card */}
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 mb-8">
          <div className="flex items-center gap-3 mb-4">
            <div
              className={`w-3 h-3 rounded-full ${
                voteProgress?.isActive
                  ? "bg-green-500 animate-pulse"
                  : voteProgress?.isTally
                    ? "bg-blue-500 animate-pulse"
                    : "bg-gray-400"
              }`}
            ></div>
            <div>
              <h2 className="text-lg font-semibold text-gray-900">
                {voteProgress?.isActive
                  ? "Omröstning aktiv"
                  : voteProgress?.isTally
                    ? "Röster räknas"
                    : "Omröstning ej aktiv"}
              </h2>
              <p className="text-gray-600">
                {voteProgress?.isActive
                  ? "Deltagare kan nu skicka sina röster"
                  : voteProgress?.isTally
                    ? "Omröstningen är stängd, resultat beräknas"
                    : "Ingen aktiv omröstning"}
              </p>
            </div>
          </div>

          {/* Progress Bar */}
          {voteProgress && (voteProgress.isActive || voteProgress.isTally) && (
            <div className="mb-4">
              <div className="flex justify-between text-sm text-gray-600 mb-2">
                <span>Röstdeltagande</span>
                <span>
                  {voteProgress.totalVotesCast}/{voteProgress.totalParticipants}
                </span>
              </div>
              <div className="w-full bg-gray-200 rounded-full h-2">
                <div
                  className={`h-2 rounded-full transition-all duration-500 ${
                    voteProgress.isTally ? "bg-blue-500" : "bg-green-500"
                  }`}
                  style={{
                    width:
                      voteProgress.totalParticipants > 0
                        ? `${(voteProgress.totalVotesCast / voteProgress.totalParticipants) * 100}%`
                        : "0%",
                  }}
                ></div>
              </div>
            </div>
          )}
        </div>

        {/* Vote Progress Summary */}
        {voteProgress && (voteProgress.isActive || voteProgress.isTally) && (
          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 mb-8">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-xl font-semibold text-gray-900">
                Röstningsöversikt
              </h3>
              <div className="text-xs text-gray-500">
                Uppdateras automatiskt
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <div className="text-center p-4 bg-gray-50 rounded-lg">
                <div className="text-2xl font-bold text-gray-900">
                  {voteProgress.totalParticipants}
                </div>
                <div className="text-sm text-gray-600">
                  Totalt antal deltagare
                </div>
              </div>

              <div className="text-center p-4 bg-blue-50 rounded-lg">
                <div className="text-2xl font-bold text-blue-900">
                  {voteProgress.totalVotesCast}
                </div>
                <div className="text-sm text-blue-700">Inkomna röster</div>
              </div>

              <div className="text-center p-4 bg-green-50 rounded-lg">
                <div className="text-2xl font-bold text-green-900">
                  {voteProgress.totalParticipants > 0
                    ? Math.round(
                        (voteProgress.totalVotesCast /
                          voteProgress.totalParticipants) *
                          100,
                      )
                    : 0}
                  %
                </div>
                <div className="text-sm text-green-700">Röstdeltagande</div>
              </div>
            </div>
          </div>
        )}

        {/* Actions */}
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-lg font-semibold text-gray-900">
                Redo att avsluta omröstningen?
              </h3>
              <p className="text-gray-600 text-sm">
                Detta kommer att stänga omröstningen och beräkna resultat
              </p>
            </div>
            <button
              onClick={handleTally}
              disabled={isLoading}
              className="px-6 py-3 bg-red-600 hover:bg-red-700 disabled:bg-red-300 text-white font-medium rounded-md shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100 disabled:cursor-not-allowed"
            >
              {isLoading ? (
                <span className="flex items-center gap-2">
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                  Beräknar resultat...
                </span>
              ) : (
                "Avsluta omröstning & räkna"
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default VotingPage;
