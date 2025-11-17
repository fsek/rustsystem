import {
  try_register,
  new_ballot_validation,
  send_vote,
} from "@/pkg/rustsystem_client";
import { withWasm } from "@/utils/wasm";
import { matchResult } from "@/result";
import type React from "react";
import { useEffect, useState } from "react";

import type { MeetingSpecsResponse } from "@/api/common/meetingSpecs";
import type { APIError } from "@/api/error";
import { VotePageDisplay } from "../voter";
import {
  getVoteProgress,
  voteProgressWatch,
  type VoteProgressResponse,
} from "@/api/common/voteProgress";

type VotingPageProps = {
  muid: string;
  uuuid: string;
  specs: MeetingSpecsResponse | undefined;
  setVotePageDisplay: React.Dispatch<React.SetStateAction<VotePageDisplay>>;
  setError: React.Dispatch<React.SetStateAction<APIError | null>>;
};

const VotingPage: React.FC<VotingPageProps> = ({
  muid,
  uuuid,
  specs,
  setVotePageDisplay,
  setError,
}) => {
  const [isRegistering, setIsRegistering] = useState(false);
  const [isVoting, setIsVoting] = useState(false);
  const [hasVoted, setHasVoted] = useState(false);
  const [candidates, setCandidates] = useState<string[]>([]);
  const [voteName, setVoteName] = useState("");
  const [selectedCandidates, setSelectedCandidates] = useState<number[]>([]);
  const [maxSelections, setMaxSelections] = useState(1);
  const [minSelections, setMinSelections] = useState(0);
  const [voteProgress, setVoteProgress] = useState<VoteProgressResponse | null>(
    null,
  );



  useEffect(() => {
    console.log("=== Voting component mounted ===");
    console.log("Checking localStorage for existing tokens:");
    console.log("validation exists:", !!localStorage.getItem("validation"));
    console.log("metadata exists:", !!localStorage.getItem("metadata"));
    console.log("voteInfo exists:", !!localStorage.getItem("voteInfo"));

    const initializeComponent = async () => {
      await checkVotingState();
      performAuth();
    };
    initializeComponent();
    fetchVoteProgress();
    const cleanup = setupVoteProgressWatch();

    // Set up periodic refresh as fallback
    const progressInterval = setInterval(fetchVoteProgress, 10000); // Every 10 seconds

    return () => {
      if (cleanup) cleanup();
      clearInterval(progressInterval);
    };
  }, []);

  // Check if user has already voted (for refresh recovery)
  const checkVotingState = async () => {
    const voteInfo = localStorage.getItem("voteInfo");

    if (voteInfo) {
      try {
        const info = JSON.parse(voteInfo);

        // Check if we're in a new voting round by fetching current progress
        try {
          const result = await getVoteProgress({});
          await new Promise((resolve) => {
            matchResult(result, {
              Ok: (progressData) => {
                if (progressData.isActive && progressData.voteName) {
                  if (info.voteName !== progressData.voteName) {
                    console.log(
                      `New voting round detected: stored="${info.voteName}", current="${progressData.voteName}" - clearing old voting state`,
                    );
                  }
                }
                resolve(undefined);
              },
              Err: (err) => {
                console.warn("Failed to check current vote progress:", err);
                resolve(undefined);
              },
            });
          });
        } catch (progressError) {
          console.warn("Failed to check current vote progress:", progressError);
        }

        setHasVoted(true);
        setVoteName(info.voteName || "Omröstning");
        setCandidates(info.candidates || []);
        setSelectedCandidates(info.selectedCandidates || []);
        setIsRegistering(false);
        console.log("Restored voting state: user has already voted");
        return true;
      } catch (error) {
        console.warn("Failed to parse vote info, clearing corrupted data");
      }
    }
    return false;
  };

  const fetchVoteProgress = async () => {
    try {
      const result = await getVoteProgress({});
      matchResult(result, {
        Ok: (progressData) => {
          console.log("Vote progress response:", progressData);
          setVoteProgress(progressData);
        },
        Err: (err) => {
          console.warn("Failed to fetch vote progress:", err);
        },
      });
    } catch (error) {
      console.warn("Error fetching vote progress:", error);
    }
  };

  const setupVoteProgressWatch = () => {
    const eventSource = voteProgressWatch();

    eventSource.onmessage = (event) => {
      if (event.data === "VoteProgressUpdated") {
        // When we get an update, fetch the latest progress
        fetchVoteProgress();
      }
    };

    eventSource.onerror = (error) => {
      console.error("Vote progress watch error:", error);
      // Reconnect after a delay
      setTimeout(() => {
        eventSource.close();
        setupVoteProgressWatch();
      }, 5000);
    };

    // Cleanup on unmount
    return () => {
      eventSource.close();
    };
  };

  const performAuth = async () => {
    try {
      console.log(
        "Already authenticated, checking for existing session data",
      );

      // If we have existing session data, use it
      const existingValidation = localStorage.getItem("validation");
      const existingMetadata = localStorage.getItem("metadata");

      console.log("Checking existing tokens after auth check:");
      console.log("validation exists:", !!existingValidation);
      console.log("metadata exists:", !!existingMetadata);
      try {
        const result = await getVoteProgress({});
        let votingIsActive = false;

        matchResult(result, {
          Ok: (progressData) => {
            votingIsActive = progressData.isActive;
          },
          Err: () => {
            // Ignore error
          },
        });

        if (votingIsActive) {
          // Additional safeguard: if user shows as "already voted" but voting is active,
          // this might be stale state from previous round
          autoRegister();
          return;
        }
      } catch (progressError) {
        console.warn("Failed to check vote progress:", progressError);
      }

      // if (existingValidation && existingMetadata) {
      //   try {
      //     const metadataValue = JSON.parse(existingMetadata);
      //     setCandidates(metadataValue.candidates || []);
      //     setVoteName(metadataValue.name || "Vote");
      //     setMaxSelections(metadataValue.max_choices || 1);
      //     setMinSelections(metadataValue.min_choices || 0);
      //     console.log("Using existing session data");
      //     return;
      //   } catch (parseError) {
      //     console.warn(
      //       "Failed to parse existing session data:",
      //       parseError,
      //     );
      //   }
      // }

      // If voting not active, show register page
      console.log("No active voting, showing register page");
      setVotePageDisplay(VotePageDisplay.Register);
      return;
    } catch (authError) {
      console.log("Not authenticated yet, proceeding with login");
    }

    // Validate UUID format
    if (!muid || !uuuid) {
      throw new Error(
        `Saknade UUID-parametrar - muid: ${muid}, uuuid: ${uuuid}`,
      );
    }
  };

  const autoRegister = async () => {
    try {
      setIsRegistering(true);

      // Check if we already have valid registration data in local storage
      const existingValidation = localStorage.getItem("validation");
      const existingMetadata = localStorage.getItem("metadata");

      if (existingValidation && existingMetadata) {
        console.log("Found existing registration data in local storage");
        try {
          const metadataValue = JSON.parse(existingMetadata);
          console.log("Existing metadata:", metadataValue);
          const storedVoteName = localStorage.getItem("currentVoteName");
          console.log("Stored vote name:", storedVoteName);

          // Check if tokens are for current round by comparing vote names
          try {
            const result = await getVoteProgress({});
            let shouldContinueWithNewRegistration = false;

            matchResult(result, {
              Ok: (progressData) => {
                console.log("Current vote progress:", progressData);
                const storedVoteName = localStorage.getItem("currentVoteName");
                console.log("Comparing:", {
                  stored: storedVoteName,
                  current: progressData.voteName,
                });
                if (progressData.voteName) {
                  if (storedVoteName === progressData.voteName) {
                    // Same voting round - use existing tokens
                    console.log(
                      "Existing tokens are for current round, using them",
                    );
                    setCandidates(metadataValue.candidates || []);
                    setVoteName(metadataValue.name || "Vote");
                    setMaxSelections(metadataValue.max_choices || 1);
                    setMinSelections(metadataValue.min_choices || 0);
                    setIsRegistering(false);
                    console.log(
                      "Successfully restored from existing registration data",
                    );
                  } else {
                    // Different voting round - clear old tokens and register for new round
                    console.log(
                      `New voting round detected: stored="${storedVoteName}", current="${progressData.voteName}"`,
                    );
                    localStorage.removeItem("validation");
                    localStorage.removeItem("metadata");
                    localStorage.removeItem("currentVoteName");
                    localStorage.removeItem("voteInfo");
                    shouldContinueWithNewRegistration = true;
                  }
                } else {
                  // No current vote name - assume tokens are valid
                  console.log(
                    "No current vote name available, using existing tokens",
                  );
                  setCandidates(metadataValue.candidates || []);
                  setVoteName(metadataValue.name || "Vote");
                  setMaxSelections(metadataValue.max_choices || 1);
                  setMinSelections(metadataValue.min_choices || 0);
                  setIsRegistering(false);
                }
              },
              Err: (err) => {
                console.warn(
                  "Failed to get vote progress for token validation:",
                  err,
                );
                // On error, use existing tokens
                setCandidates(metadataValue.candidates || []);
                setVoteName(metadataValue.name || "Vote");
                setMaxSelections(metadataValue.max_choices || 1);
                setMinSelections(metadataValue.min_choices || 0);
                setIsRegistering(false);
              },
            });

            // If we need to continue with new registration, exit here to proceed below
            if (shouldContinueWithNewRegistration) {
              // Continue with new registration below
            } else {
              return;
            }
          } catch (progressError) {
            console.warn("Error during vote progress check:", progressError);
            // On error, use existing tokens
            setCandidates(metadataValue.candidates || []);
            setVoteName(metadataValue.name || "Vote");
            setMaxSelections(metadataValue.max_choices || 1);
            setMinSelections(metadataValue.min_choices || 0);
            setIsRegistering(false);
            return;
          }
        } catch (parseError) {
          console.warn(
            "Failed to parse existing registration data, will re-register:",
            parseError,
          );
        }
      }

      const res = await withWasm(async () => await try_register(muid, uuuid));

      console.log("Registration response:", {
        is_valid: res.is_valid(),
        is_successful: res.is_successful(),
        has_metadata: !!res.metadata(),
      });

      if (res.is_successful()) {
        setHasVoted(false);
      } else {
        // Assume already voted
        setHasVoted(true);
      }
      if (res.is_valid() && res.is_successful()) {
        const validation = await withWasm(async () =>
          new_ballot_validation(res.proof(), res.token(), res.signature()),
        );
        const metadata = res.metadata();

        if (metadata) {
          const validationValue = validation.toValue();
          const metadataValue = metadata.toValue();

          console.log("Saving tokens to localStorage:");
          console.log("validation:", validationValue);
          console.log("metadata:", metadataValue);

          localStorage.setItem("validation", JSON.stringify(validationValue));
          localStorage.setItem("metadata", JSON.stringify(metadataValue));

          // Store vote name separately since it's not part of ballot metadata
          try {
            const currentProgress = await getVoteProgress({});
            matchResult(currentProgress, {
              Ok: (progressData) => {
                if (progressData.voteName) {
                  localStorage.setItem(
                    "currentVoteName",
                    progressData.voteName,
                  );
                  console.log(
                    "Stored current vote name:",
                    progressData.voteName,
                  );
                }
              },
              Err: (err) => {
                console.warn("Failed to get vote name for storage:", err);
              },
            });
          } catch (error) {
            console.warn("Error getting vote name for storage:", error);
          }

          console.log("Registration successful, tokens saved to localStorage");
          setCandidates(metadataValue.candidates || []);
          setVoteName(metadataValue.name || "Vote");
          setMaxSelections(metadataValue.max_choices || 1);
          setMinSelections(metadataValue.min_choices || 0);
        } else {
          throw new Error("Ingen metadata mottagen från registrering");
        }

        setIsRegistering(false);
      } else {
        const errorMsg = `Registrering misslyckades - Giltig: ${res.is_valid()}, Framgångsrik: ${res.is_successful()}`;
        console.error(errorMsg);
        throw new Error(errorMsg);
      }
    } catch (error) {
      console.error("Auto-registration error:", error);
      const errorMessage =
        error instanceof Error ? error.message : String(error);

      // Check if this is an AlreadyRegistered error (refresh scenario)
      if (
        errorMessage.includes("AlreadyRegistered") ||
        errorMessage.includes("already registered") ||
        errorMessage.includes("Empty") ||
        errorMessage.includes("409")
      ) {
        console.log("User already registered - refresh scenario detected");
        // For refresh scenarios, show helpful message instead of broken register button
        // setError({
        //   code: "AlreadyRegistered",
        //   message:
        //     "Du är redan registrerad för denna omröstning. Dina röstnings-tokens har tyvärr förlorats vid siduppdateringen. För säkerhets skull kan du inte rösta efter en refresh under pågående omröstning.",
        //   httpStatus: 409,
        //   timestamp: new Date().toISOString(),
        //   endpoint: { method: "POST", path: "/api/voter/register" },
        // });
        // setVotePageDisplay(VotePageDisplay.RegistrationFail);
      } else {
        // For other errors, show register page
        console.log(
          "Auto-registration failed with other error, showing register page",
        );
        setVotePageDisplay(VotePageDisplay.Register);
      }
    } finally {
      setIsRegistering(false);
    }
  };

  const handleCandidateToggle = (candidateIndex: number) => {
    if (hasVoted) return;

    setSelectedCandidates((prev) => {
      const isSelected = prev.includes(candidateIndex);

      if (isSelected) {
        // Remove selection
        return prev.filter((idx) => idx !== candidateIndex);
      } else {
        // Add selection if under max limit
        if (prev.length < maxSelections) {
          return [...prev, candidateIndex].sort();
        }
        return prev;
      }
    });
  };

  const handleCastVote = async () => {
    if (isVoting || hasVoted) return;

    try {
      setIsVoting(true);

      const validationData = localStorage.getItem("validation");
      const metadataData = localStorage.getItem("metadata");

      if (!validationData || !metadataData) {
        throw new Error("Registreringsdata saknas");
      }

      const validation = JSON.parse(validationData);
      const metadata = JSON.parse(metadataData);

      // Send selected candidates or null for blank vote
      const choice = selectedCandidates.length > 0 ? selectedCandidates : null;

      await withWasm(async () => await send_vote(metadata, choice, validation));

      setHasVoted(true);
      setIsVoting(false);

      // Store voting completion state for refresh recovery
      const voteInfo = {
        voteName,
        candidates,
        selectedCandidates,
        votedAt: new Date().toISOString(),
      };
      localStorage.setItem("voteInfo", JSON.stringify(voteInfo));
    } catch (error) {
      console.error(error);
      setError({
        code: "VotingError",
        message: "Misslyckades att skicka röst",
        httpStatus: 500,
        timestamp: new Date().toISOString(),
        endpoint: { method: "POST", path: "/api/voter/submit" },
      });
      setIsVoting(false);
    }
  };

  const canVote = () => {
    return (
      selectedCandidates.length >= minSelections &&
      selectedCandidates.length <= maxSelections
    );
  };

  if (isRegistering) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mx-auto mb-4"></div>
          <h2 className="text-xl font-semibold text-gray-900 mb-2">
            Förbereder din röstsedel
          </h2>
          <p className="text-gray-600">
            Vänligen vänta medan vi ställer in din säkra röstningssession...
          </p>
        </div>
      </div>
    );
  }

  if (hasVoted) {
    return (
      <div className="min-h-screen bg-gray-50">
        <div className="max-w-4xl mx-auto px-4 py-8">
          {/* Admin Navigation */}

          {/* Header */}
          <div className="text-center mb-8">
            <h1 className="text-3xl font-bold text-gray-900 mb-2">
              Röst skickad framgångsrikt
            </h1>
            <div className="flex items-center justify-center gap-4 text-sm text-gray-600">
              <span className="flex items-center gap-1">
                ✅ Röst registrerad
              </span>
              <span className="flex items-center gap-1">🔒 Säker & anonym</span>
            </div>
          </div>

          {/* Progress Bar */}
          {voteProgress && voteProgress.isActive && (
            <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 mb-6">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-semibold text-gray-900">
                  Omröstningsframsteg
                </h3>
                <div className="text-sm text-gray-600">
                  {voteProgress.totalVotesCast} av{" "}
                  {voteProgress.totalParticipants} röster avgivna
                </div>
              </div>
              <div className="w-full bg-gray-200 rounded-full h-3">
                <div
                  className="bg-blue-600 h-3 rounded-full transition-all duration-300 ease-out"
                  style={{
                    width:
                      voteProgress.totalParticipants > 0
                        ? `${Math.min((voteProgress.totalVotesCast / voteProgress.totalParticipants) * 100, 100)}%`
                        : "0%",
                  }}
                ></div>
              </div>
              <div className="flex justify-between text-sm text-gray-500 mt-2">
                <span>0 röster</span>
                <span>{voteProgress.totalParticipants} röster</span>
              </div>
            </div>
          )}

          {/* Success Card */}
          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 text-center">
            <div className="w-16 h-16 bg-green-100 rounded-full flex items-center justify-center mx-auto mb-4">
              <div className="w-8 h-8 text-green-600">✓</div>
            </div>
            <h2 className="text-xl font-semibold text-gray-900 mb-2">
              Tack för din röst!
            </h2>
            <p className="text-gray-600 mb-4">
              Din röst har säkert registrerats. Resultat kommer att vara
              tillgängliga när omröstningen stängs.
            </p>
            <div className="text-sm text-gray-500">
              Du röstade i: <span className="font-medium">{voteName}</span>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="max-w-4xl mx-auto px-4 py-8">
        {/* Admin Navigation */}

        {/* Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">{voteName}</h1>
          <div className="flex items-center justify-center gap-4 text-sm text-gray-600">
            <span className="flex items-center gap-1">
              👥 {specs?.participants || 0} deltagare
            </span>
            <span className="flex items-center gap-1">🗳️ Omröstning aktiv</span>
          </div>
        </div>

        {/* Progress Bar */}
        {voteProgress && voteProgress.isActive && (
          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 mb-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-gray-900">
                Omröstningsframsteg
              </h3>
              <div className="text-sm text-gray-600">
                {voteProgress.totalVotesCast} av{" "}
                {voteProgress.totalParticipants} röster avgivna
              </div>
            </div>
            <div className="w-full bg-gray-200 rounded-full h-3">
              <div
                className="bg-blue-600 h-3 rounded-full transition-all duration-300 ease-out"
                style={{
                  width:
                    voteProgress.totalParticipants > 0
                      ? `${Math.min((voteProgress.totalVotesCast / voteProgress.totalParticipants) * 100, 100)}%`
                      : "0%",
                }}
              ></div>
            </div>
            <div className="flex justify-between text-sm text-gray-500 mt-2">
              <span>0 röster</span>
              <span>{voteProgress.totalParticipants} röster</span>
            </div>
          </div>
        )}

        {/* Voting Instructions */}
        <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-8">
          <div className="flex items-start gap-3">
            <div className="text-blue-500 mt-0.5">ℹ️</div>
            <div>
              <h3 className="font-medium text-blue-900 mb-1">Avge din röst</h3>
              <p className="text-blue-800 text-sm">
                {maxSelections === 0
                  ? "Endast blanka röster tillåts för denna omröstning."
                  : maxSelections === 1
                    ? "Välj en kandidat nedan, eller avge en blank röst."
                    : `Välj upp till ${maxSelections} kandidater nedan, eller avge en blank röst.`}
                Din röst är anonym och säker.
              </p>
            </div>
          </div>
        </div>

        {/* Vote Selection Status */}
        <div className="bg-gray-50 border border-gray-200 rounded-lg p-4 mb-6">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium text-gray-700">
                Valda: {selectedCandidates.length} av{" "}
                {maxSelections === 0 ? "0" : maxSelections}
              </span>
              {selectedCandidates.length > 0 && (
                <button
                  onClick={() => setSelectedCandidates([])}
                  className="text-xs text-blue-600 hover:text-blue-800"
                >
                  Rensa alla
                </button>
              )}
            </div>
            <div className="flex items-center gap-2">
              {selectedCandidates.length === 0 ? (
                <span className="text-xs text-gray-500 bg-yellow-100 px-2 py-1 rounded">
                  Blank röst
                </span>
              ) : (
                <span className="text-xs text-green-700 bg-green-100 px-2 py-1 rounded">
                  {selectedCandidates.length} valda
                </span>
              )}
            </div>
          </div>
        </div>

        {/* Candidates */}
        {maxSelections > 0 && (
          <div className="space-y-3 mb-8">
            {candidates.map((candidate, index) => {
              const isSelected = selectedCandidates.includes(index);
              const canSelect =
                selectedCandidates.length < maxSelections || isSelected;

              return (
                <button
                  key={index}
                  onClick={() => handleCandidateToggle(index)}
                  disabled={!canSelect || hasVoted}
                  className={`w-full p-6 rounded-lg border-2 text-left transition-all duration-200 ${isSelected
                    ? "border-blue-500 bg-blue-50"
                    : canSelect
                      ? "border-gray-200 bg-white hover:border-blue-300 hover:bg-blue-50 active:scale-[0.99]"
                      : "border-gray-200 bg-gray-50 opacity-50 cursor-not-allowed"
                    }`}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-4">
                      <div
                        className={`w-12 h-12 rounded-full flex items-center justify-center font-semibold ${isSelected
                          ? "bg-blue-500 text-white"
                          : "bg-gray-100 text-gray-600"
                          }`}
                      >
                        {isSelected ? "✓" : candidate.charAt(0).toUpperCase()}
                      </div>
                      <div>
                        <h3 className="text-lg font-semibold text-gray-900">
                          {candidate}
                        </h3>
                        <p className="text-sm text-gray-600">
                          {isSelected ? "Vald" : "Klicka för att välja"}
                        </p>
                      </div>
                    </div>
                    <div
                      className={isSelected ? "text-blue-500" : "text-gray-400"}
                    >
                      {isSelected ? (
                        <div className="w-6 h-6 bg-blue-500 text-white rounded-full flex items-center justify-center">
                          <span className="text-xs">✓</span>
                        </div>
                      ) : (
                        <div className="w-6 h-6 border-2 border-gray-300 rounded-full"></div>
                      )}
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
        )}

        {/* Cast Vote Button */}
        <div className="bg-white border border-gray-200 rounded-lg p-6">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h3 className="text-lg font-semibold text-gray-900">
                Redo att rösta?
              </h3>
              <p className="text-sm text-gray-600">
                {selectedCandidates.length === 0
                  ? "Du avger en blank röst"
                  : `Du har valt ${selectedCandidates.length} kandidat${selectedCandidates.length > 1 ? "er" : ""}`}
              </p>
            </div>
          </div>

          {selectedCandidates.length > 0 && (
            <div className="mb-4 p-3 bg-blue-50 rounded-lg">
              <p className="text-sm font-medium text-blue-900 mb-1">
                Dina val:
              </p>
              <div className="flex flex-wrap gap-2">
                {selectedCandidates.map((idx) => (
                  <span
                    key={idx}
                    className="bg-blue-100 text-blue-800 px-2 py-1 rounded text-sm"
                  >
                    {candidates[idx]}
                  </span>
                ))}
              </div>
            </div>
          )}

          <button
            onClick={handleCastVote}
            disabled={!canVote() || isVoting || hasVoted}
            className="w-full py-4 bg-green-600 hover:bg-green-700 disabled:bg-gray-300 text-white font-semibold rounded-lg transition-all duration-200 disabled:cursor-not-allowed"
          >
            {isVoting ? (
              <div className="flex items-center justify-center gap-2">
                <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
                <span>Skickar röst...</span>
              </div>
            ) : (
              <span>
                {selectedCandidates.length === 0
                  ? "Avge blank röst"
                  : "Avge röst"}
              </span>
            )}
          </button>

          {!canVote() && (
            <p className="text-xs text-red-600 mt-2 text-center">
              {selectedCandidates.length < minSelections
                ? `Du måste välja minst ${minSelections} kandidat${minSelections > 1 ? "er" : ""}`
                : `Du kan välja högst ${maxSelections} kandidat${maxSelections > 1 ? "er" : ""}`}
            </p>
          )}
        </div>
      </div>

      {/* Footer Info */}
      <div className="mt-8 text-center text-sm text-gray-500">
        <p>
          Din röst är kryptografiskt säker och anonym. När den väl har skickats
          kan den inte ändras.
        </p>
      </div>
    </div>
  );
};

export default VotingPage;
