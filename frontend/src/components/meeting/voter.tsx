import RegisterPage from "@/components/meeting/vote-page/register";
import VotingPage from "@/components/meeting/vote-page/voting";
import React, { type ReactElement, useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import {
  MeetingSpecs,
  type MeetingSpecsRequest,
  type MeetingSpecsResponse,
  meetingSpecsWatch,
} from "@/api/common/meetingSpecs";
import { voteStateWatch } from "@/api/common/state";
import type { APIError } from "@/api/error";
import { matchResult } from "@/result";
import ErrorHandler from "../error";

type VoterPageProps = {
  muid: any;
  uuid: any;
};

export const VotePageDisplay = {
  // Error pages. Something went wrong.
  RegistrationFail: 1,
  VoteFail: 2,

  // Non-error info pages
  VoteFinished: 3,
  AlreadyVoted: 4,

  // Main functional pages
  Wait: 5,
  Register: 6,
  Voting: 7,
} as const;
export type VotePageDisplay =
  (typeof VotePageDisplay)[keyof typeof VotePageDisplay];

const VoterPage: React.FC<VoterPageProps> = ({ muid, uuid }) => {
  const [voteStateEvent, setVoteStateEvent] = useState<EventSource | null>(
    null,
  );
  const [currentVotePageDisplay, setVotePageDisplay] =
    useState<VotePageDisplay>(VotePageDisplay.Wait);
  const [error, setError] = useState<APIError | null>(null);
  const [specs, setSpecs] = useState<MeetingSpecsResponse | undefined>(
    undefined,
  );
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [pollingInterval, setPollingInterval] = useState<number | null>(null);

  // Check if already authenticated and vote state, but don't fail if not
  useEffect(() => {
    const checkAuthAndVoteState = async () => {
      try {
        // Strategy: Try vote-active first as it's faster and tells us both auth state and vote state
        console.log("Checking vote state and authentication...");
        const voteActiveCheck = await fetch("/api/common/vote-active", {
          method: "GET",
          credentials: "include",
        });

        if (voteActiveCheck.ok) {
          const voteData = await voteActiveCheck.json();
          console.log("Vote state data:", voteData);

          // We're authenticated and got vote state
          setIsAuthenticated(true);

          if (voteData.isActive) {
            console.log(
              "✅ Voting is active - transitioning immediately to voting page",
            );
            setVotePageDisplay(VotePageDisplay.Voting);
          } else {
            console.log("📝 Voting not active - staying on wait page");
            setVotePageDisplay(VotePageDisplay.Wait);
          }

          // Set up real-time monitoring
          startVoteStateWatching();
          fetchMeetingSpecs();
          return;
        }

        // If vote-active failed with 401, we're not authenticated
        if (voteActiveCheck.status === 401) {
          console.log("❌ Not authenticated - starting polling mode");
          setIsAuthenticated(false);
          startPollingForVoteState();
          return;
        }

        // For other errors, fall back to auth check
        console.log("🔄 Vote-active failed, trying auth check...");
        const authCheck = await fetch("/api/auth/meeting", {
          method: "POST",
          credentials: "include",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ muuid: muid }),
        });

        if (authCheck.ok) {
          console.log("✅ Authenticated via auth check");
          setIsAuthenticated(true);
          if (pollingInterval) {
            clearInterval(pollingInterval);
            setPollingInterval(null);
          }
          startVoteStateWatching();
          fetchMeetingSpecs();
        } else {
          console.log("❌ Authentication failed - starting polling");
          setIsAuthenticated(false);
          startPollingForVoteState();
        }
      } catch (error) {
        console.log("🚫 Connection error - starting polling mode:", error);
        setIsAuthenticated(false);
        startPollingForVoteState();
      }
    };

    checkAuthAndVoteState();
  }, []);

  // Start polling for vote state changes for non-authenticated users
  const startPollingForVoteState = () => {
    console.log("Starting polling for vote state changes");

    // Clear any existing polling
    if (pollingInterval) {
      clearInterval(pollingInterval);
    }

    // Optimized polling with immediate check and faster intervals
    const pollVoteState = async () => {
      try {
        const response = await fetch("/api/common/vote-active", {
          method: "GET",
          credentials: "include",
        });

        if (response.status === 401) {
          console.log("⏳ Polling: still not authenticated, continuing...");
          return;
        }

        if (response.ok) {
          const data = await response.json();
          setIsAuthenticated(true);

          if (data.isActive) {
            console.log(
              "🎉 Polling: voting detected! Transitioning to voting page",
            );
            setVotePageDisplay(VotePageDisplay.Voting);

            // Stop polling and switch to real-time monitoring
            if (pollingInterval) {
              clearInterval(pollingInterval);
              setPollingInterval(null);
            }
            startVoteStateWatching();
            fetchMeetingSpecs();
            return;
          }

          console.log("📝 Polling: authenticated but voting not active yet");
          // User became authenticated but voting not started yet
          // Stop polling and switch to EventSource for instant updates
          if (pollingInterval) {
            clearInterval(pollingInterval);
            setPollingInterval(null);
          }
          startVoteStateWatching();
          fetchMeetingSpecs();
        }
      } catch (error) {
        console.log("❌ Polling error:", error);
      }
    };

    // Immediate first check for instant detection
    console.log("🔍 Starting immediate vote state check...");
    pollVoteState();

    // Then poll every 1 second for very fast detection if immediate check doesn't succeed
    const interval = setInterval(pollVoteState, 1000);
    setPollingInterval(interval);
  };

  const startVoteStateWatching = () => {
    if (voteStateEvent) {
      console.log("Closing existing vote state event source");
      voteStateEvent.close();
    }

    console.log("Creating new vote state event source");
    const newVoteStateEvent = voteStateWatch();
    setVoteStateEvent(newVoteStateEvent);

    newVoteStateEvent.onopen = () => {
      console.log("Vote state watch connection opened successfully");
    };

    newVoteStateEvent.onmessage = (event) => {
      console.log("Vote state event received:", event.data);
      console.log("Current display state:", currentVotePageDisplay);

      if (event.data === "Voting") {
        console.log("Transitioning to voting page");
        setVotePageDisplay(VotePageDisplay.Voting);
      } else if (event.data === "Creation" || event.data === "Tally") {
        console.log("Transitioning to wait page");
        // Clear voting state when voting phase ends
        sessionStorage.removeItem("hasVoted");
        sessionStorage.removeItem("voteInfo");
        console.log("Cleared voting state for new voting round");
        setVotePageDisplay(VotePageDisplay.Wait);
      }
    };

    newVoteStateEvent.onerror = (error) => {
      console.log("Vote state watch error:", error);
      console.log("Authentication status:", isAuthenticated);
      // If EventSource fails, fallback to polling
      setTimeout(() => {
        console.log("EventSource failed, falling back to polling");
        startPollingForVoteState();
      }, 1000);
    };
  };

  const fetchMeetingSpecs = () => {
    MeetingSpecs({} as MeetingSpecsRequest).then((result) => {
      matchResult(result, {
        Ok: (specsData) => {
          setSpecs(specsData);
        },
        Err: (err) => {
          console.error("Failed to fetch meeting specs:", err);
        },
      });
    });
  };

  // Watch for meeting specs updates if authenticated
  useEffect(() => {
    if (!isAuthenticated) {
      return;
    }

    const specsEvent = meetingSpecsWatch();

    specsEvent.onmessage = (event) => {
      if (event.data === "NewData") {
        fetchMeetingSpecs();
      }
    };

    specsEvent.onerror = (error) => {
      console.log("Meeting specs watch error:", error);
    };

    return () => {
      specsEvent.close();
    };
  }, [isAuthenticated]);

  // Cleanup vote state event and polling on unmount
  useEffect(() => {
    return () => {
      if (voteStateEvent) {
        voteStateEvent.close();
      }
      if (pollingInterval) {
        clearInterval(pollingInterval);
      }
    };
  }, [voteStateEvent, pollingInterval]);

  if (error) {
    return <ErrorHandler error={error} />;
  }
  switch (currentVotePageDisplay) {
    case VotePageDisplay.Wait:
      return WaitPage(specs, isAuthenticated);
    case VotePageDisplay.Register:
      return (
        <RegisterPage
          muid={muid}
          uuid={uuid}
          setVotePageDisplay={setVotePageDisplay}
        />
      );
    case VotePageDisplay.Voting:
      return (
        <VotingPage
          muid={muid}
          uuid={uuid}
          specs={specs}
          setVotePageDisplay={setVotePageDisplay}
          setError={setError}
        />
      );
    case VotePageDisplay.AlreadyVoted:
      return AlreadyVotedPage(specs);
    default:
      setVotePageDisplay(VotePageDisplay.Wait);
  }
};

function WaitPage(
  specs: MeetingSpecsResponse | undefined,
  isAuthenticated: boolean,
): ReactElement {
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
              👥 {specs?.participants || 0} deltagare
            </span>
            <span className="flex items-center gap-1">⏳ Väntar på start</span>
          </div>
        </div>

        {/* Status Card */}
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 mb-8">
          <div className="flex items-center gap-3">
            <div className="w-3 h-3 bg-yellow-500 rounded-full animate-pulse"></div>
            <div>
              <h2 className="text-lg font-semibold text-gray-900">
                Väntar på omröstning
              </h2>
              <p className="text-gray-600">
                Värden förbereder omröstningen. Vänligen vänta.
              </p>
            </div>
          </div>
        </div>

        {/* Info Notice for Non-authenticated Users */}
        {!isAuthenticated && (
          <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-8">
            <div className="flex items-start gap-3">
              <div className="text-blue-500 mt-0.5">ℹ️</div>
              <div>
                <h3 className="font-medium text-blue-900 mb-1">
                  Välkommen till mötet
                </h3>
                <p className="text-blue-800 text-sm">
                  När omröstningen startar kommer du automatiskt vidare till
                  röstningssidan. Du behöver inte göra något just nu.
                </p>
              </div>
            </div>
          </div>
        )}

        {/* Connection Status for Authenticated Users */}
        {isAuthenticated && (
          <div className="bg-green-50 border border-green-200 rounded-lg p-4 mb-8">
            <div className="flex items-start gap-3">
              <div className="text-green-500 mt-0.5">✅</div>
              <div>
                <h3 className="font-medium text-green-900 mb-1">
                  Ansluten till mötet
                </h3>
                <p className="text-green-800 text-sm">
                  Du är redo att delta. Väntar på att omröstningen ska starta.
                </p>
              </div>
            </div>
          </div>
        )}

        {/* Agenda Display */}
        {specs?.agenda && (
          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
            <h3 className="text-xl font-semibold text-gray-900 mb-4">
              Mötesagenda
            </h3>
            <style>
              {`
                .agenda-content ol {
                  padding-inline-start: 0;
                  margin-block-end: 1em;
                  counter-reset: section;
                }

                .agenda-content ol li {
                  display: block;
                }

                .agenda-content ol > li {
                  counter-increment: section;
                }

                .agenda-content ol > li::before {
                  content: "§ " counter(section) ". ";
                  font-weight: bold;
                }

                .agenda-content ol > li > ol {
                  counter-reset: subsection;
                }

                .agenda-content ol > li > ol > li {
                  counter-increment: subsection;
                }

                .agenda-content ol > li > ol > li::before {
                  content: "§ " counter(section) "." counter(subsection) " ";
                  margin-inline-end: 0.2em;
                }

                .agenda-content ol > li > ol > li > ol {
                  counter-reset: subsubsection;
                }

                .agenda-content ol > li > ol > li > ol > li {
                  counter-increment: subsubsection;
                }

                .agenda-content ol > li > ol > li > ol > li::before {
                  content: "§ " counter(section) "." counter(subsection) "." counter(subsubsection) " ";
                }

                .agenda-content ol ol {
                  margin-block-end: 0;
                  padding-inline-start: 1.5em;
                }
              `}
            </style>
            <div className="agenda-content prose prose-gray max-w-none">
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={{
                  h1: ({ children }) => (
                    <h1 className="text-2xl font-bold text-gray-900 mt-6 mb-4 first:mt-0">
                      {children}
                    </h1>
                  ),
                  h2: ({ children }) => (
                    <h2 className="text-xl font-semibold text-gray-800 mt-5 mb-3">
                      {children}
                    </h2>
                  ),
                  h3: ({ children }) => (
                    <h3 className="text-lg font-medium text-gray-800 mt-4 mb-2">
                      {children}
                    </h3>
                  ),
                  p: ({ children }) => (
                    <p className="text-gray-700 mb-3 leading-relaxed">
                      {children}
                    </p>
                  ),
                  ul: ({ children }) => (
                    <ul className="list-disc list-inside text-gray-700 mb-3 space-y-1">
                      {children}
                    </ul>
                  ),
                  ol: ({ children }) => (
                    <ol className="text-gray-700 mb-3 space-y-1">{children}</ol>
                  ),
                  li: ({ children }) => (
                    <li className="text-gray-700">{children}</li>
                  ),
                  blockquote: ({ children }) => (
                    <blockquote className="border-l-4 border-blue-200 pl-4 py-2 mb-3 bg-blue-50 text-gray-700 italic">
                      {children}
                    </blockquote>
                  ),
                  code: ({ children, className }) => {
                    const isBlock = className?.includes("language-");
                    if (isBlock) {
                      return (
                        <pre className="bg-gray-100 rounded p-3 mb-3 overflow-x-auto">
                          <code className="text-sm text-gray-800 font-mono">
                            {children}
                          </code>
                        </pre>
                      );
                    }
                    return (
                      <code className="bg-gray-100 px-1 py-0.5 rounded text-sm font-mono text-gray-800">
                        {children}
                      </code>
                    );
                  },
                  strong: ({ children }) => (
                    <strong className="font-semibold text-gray-900">
                      {children}
                    </strong>
                  ),
                  em: ({ children }) => (
                    <em className="italic text-gray-700">{children}</em>
                  ),
                  hr: () => <hr className="border-gray-300 my-6" />,
                  table: ({ children }) => (
                    <div className="overflow-x-auto mb-4">
                      <table className="min-w-full border border-gray-300">
                        {children}
                      </table>
                    </div>
                  ),
                  thead: ({ children }) => (
                    <thead className="bg-gray-50">{children}</thead>
                  ),
                  tbody: ({ children }) => (
                    <tbody className="bg-white">{children}</tbody>
                  ),
                  tr: ({ children }) => (
                    <tr className="border-b border-gray-200">{children}</tr>
                  ),
                  th: ({ children }) => (
                    <th className="px-4 py-2 text-left font-semibold text-gray-900 border-r border-gray-300 last:border-r-0">
                      {children}
                    </th>
                  ),
                  td: ({ children }) => (
                    <td className="px-4 py-2 text-gray-700 border-r border-gray-300 last:border-r-0">
                      {children}
                    </td>
                  ),
                }}
              >
                {specs.agenda}
              </ReactMarkdown>
            </div>
          </div>
        )}

        {!specs?.agenda && (
          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 text-center">
            <div className="text-gray-400 mb-2">📋</div>
            <p className="text-gray-500">
              Ingen agenda har ställts in för detta möte
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

function AlreadyVotedPage(
  specs: MeetingSpecsResponse | undefined,
): ReactElement {
  return (
    <div className="min-h-screen bg-gray-50">
      <div className="max-w-4xl mx-auto px-4 py-8">
        {/* Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">
            {specs?.title || "Möte"}
          </h1>
          <div className="flex items-center justify-center gap-4 text-sm text-gray-600">
            <span className="flex items-center gap-1">✅ Redan röstat</span>
            <span className="flex items-center gap-1">🔒 Säker & anonym</span>
          </div>
        </div>

        {/* Already Voted Card */}
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 mb-8">
          <div className="flex items-center gap-3">
            <div className="w-3 h-3 bg-green-500 rounded-full"></div>
            <div>
              <h2 className="text-lg font-semibold text-gray-900">
                Du har redan röstat
              </h2>
              <p className="text-gray-600">
                Din röst har redan registrerats i denna omröstning. Resultaten
                kommer att vara tillgängliga när omröstningen stängs.
              </p>
            </div>
          </div>
        </div>

        {/* Info Notice */}
        <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-8">
          <div className="flex items-start gap-3">
            <div className="text-blue-500 mt-0.5">ℹ️</div>
            <div>
              <h3 className="font-medium text-blue-900 mb-1">
                Tack för din deltagande
              </h3>
              <p className="text-blue-800 text-sm">
                Du kan stänga denna flik. Om du tror att detta är ett fel,
                kontakta administratören.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default VoterPage;
