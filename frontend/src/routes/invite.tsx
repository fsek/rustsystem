import { Auth, type AuthMeetingRequest } from "@/api/auth";
import {
  VoteActive,
  type VoteActiveRequest,
  voteStateWatch,
} from "@/api/common/state";
import type { APIError } from "@/api/error";
import { startInviteWait } from "@/api/host/inviteEvent";
import {
  type NewVoterRequest,
  newVoter,
  startInvite,
  type startInviteRequest,
} from "@/api/host/newVoter";
import { removeAll, type RemoveAllRequest } from "@/api/host/removeAll";
import { type ResetLoginRequest, resetLogin } from "@/api/host/resetLogin";
import { VoterList, type VoterListRequest } from "@/api/host/voterList";
import ErrorHandler from "@/components/error";
import { matchResult } from "@/result";
import { rankItem } from "@tanstack/match-sorter-utils";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import {
  type ColumnFiltersState,
  type SortingState,
  type VisibilityState,
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";
import { useEffect, useState } from "react";
import "@/colors.css";

type SearchParams = {
  muid: string;
};

export const Route = createFileRoute("/invite")({
  validateSearch: (search): SearchParams => {
    return {
      muid: (search.muid as string) ?? "",
    };
  },
  component: RouteComponent,
});

interface Voter {
  uuid: string;
  name: string;
  registeredAt: string;
  loggedIn: boolean;
  isHost: boolean;
}

const columnHelper = createColumnHelper<Voter>();

function RouteComponent() {
  const navigate = useNavigate();
  const search = Route.useSearch();
  const muid = search.muid;

  const [voters, setVoters] = useState<Voter[]>([]);
  const [isVotingActive, setIsVotingActive] = useState(false);

  const [newVoterName, setNewVoterName] = useState("");
  const [isNewVoterAdmin, setIsNewVoterAdmin] = useState(false);
  const [selectedVoter, setSelectedVoter] = useState<Voter | null>(null);
  const [nameCollisionError, setNameCollisionError] = useState<string | null>(
    null,
  );
  const [qrCodeUrl, setQrCodeUrl] = useState<string | null>(null);
  const [isQrModalOpen, setIsQrModalOpen] = useState(false);

  // Handle escape key for QR modal
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isQrModalOpen) {
        setIsQrModalOpen(false);
      }
    };

    if (isQrModalOpen) {
      document.addEventListener("keydown", handleEscape);
      document.body.style.overflow = "hidden"; // Prevent background scroll
    }

    return () => {
      document.removeEventListener("keydown", handleEscape);
      document.body.style.overflow = "unset";
    };
  }, [isQrModalOpen]);
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<APIError | null>(null);
  const [inviteReady, setInviteReady] = useState(false);
  const [globalFilter, setGlobalFilter] = useState("");
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const [sorting, setSorting] = useState<SortingState>([]);
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({});
  const [currentUserUuid, setCurrentUserUuid] = useState<string | null>(null);

  const checkVoteState = async () => {
    try {
      const result = await VoteActive({} as VoteActiveRequest);
      matchResult(result, {
        Ok: (res) => {
          setIsVotingActive(res.isActive);
        },
        Err: (err) => {
          console.error("Failed to check vote state:", err);
        },
      });
    } catch (error) {
      console.error("Error checking vote state:", error);
    }
  };

  useEffect(() => {
    // Initial check of vote state
    checkVoteState();

    // Get current user's UUID
    Auth({ muuid: muid } as AuthMeetingRequest).then((result) => {
      matchResult(result, {
        Ok: (res) => {
          setCurrentUserUuid(res.uuid);
        },
        Err: (err) => {
          console.error("Failed to get current user:", err);
        },
      });
    });

    // Watch for vote state changes
    const voteStateWatcher = voteStateWatch();
    voteStateWatcher.onmessage = (event) => {
      if (event.data === "Voting") {
        setIsVotingActive(true);
      } else if (event.data === "Creation" || event.data === "Tally") {
        setIsVotingActive(false);
      }
    };

    return () => {
      voteStateWatcher.close();
    };
  }, []);

  const fetchVoters = async () => {
    const result = await VoterList({} as VoterListRequest);
    matchResult(result, {
      Ok: (response) => {
        const votersWithStatus = response.voters.map((voter) => ({
          uuid: voter.uuid,
          name: voter.name,
          registeredAt: voter.registeredAt,
          loggedIn: voter.loggedIn,
          isHost: voter.isHost,
        }));
        setVoters(votersWithStatus);
      },
      Err: (err) => setError(err),
    });
  };

  useEffect(() => {
    // Fetch initial voter list
    fetchVoters();
    // Initialize invite system
    const inviteEvent = startInviteWait();
    inviteEvent.onmessage = (event) => {
      if (event.data === "Ready") {
        setInviteReady(true);
      }
    };

    inviteEvent.onopen = () => {
      startInvite({} as startInviteRequest).then((result) => {
        matchResult(result, {
          Ok: () => {},
          Err: (err) => setError(err),
        });
      });
    };

    return () => {
      inviteEvent.close();
    };
  }, []);

  const generateQrCodeForNewVoter = async (
    voterName: string,
    isAdmin = false,
  ) => {
    if (!inviteReady) {
      setError({
        code: "InviteNotReady",
        message: "Invite system not ready yet",
        httpStatus: 400,
        timestamp: new Date().toISOString(),
        endpoint: { method: "POST", path: "/api/host/new-voter" },
      });
      return;
    }

    setIsGenerating(true);
    setQrCodeUrl(null);
    setNameCollisionError(null);

    const result = await newVoter({
      voterName: voterName,
      isHost: isAdmin,
    } as NewVoterRequest);

    matchResult(result, {
      Ok: (res) => {
        const url = URL.createObjectURL(res.blob);
        setQrCodeUrl(url);
        setIsGenerating(false);
        // Refresh voter list after adding and update selectedVoter with real data
        VoterList({} as VoterListRequest).then((updatedVoters) => {
          matchResult(updatedVoters, {
            Ok: (response) => {
              const votersWithStatus = response.voters.map((voter) => ({
                uuid: voter.uuid,
                name: voter.name,
                registeredAt: voter.registeredAt,
                loggedIn: voter.loggedIn,
                isHost: voter.isHost,
              }));
              setVoters(votersWithStatus);
              // Update selectedVoter with the real voter data from backend
              const newlyCreatedVoter = votersWithStatus.find(
                (v) => v.name === voterName,
              );
              if (newlyCreatedVoter) {
                setSelectedVoter(newlyCreatedVoter);
                // Clear form only on successful addition
                setNewVoterName("");
                setIsNewVoterAdmin(false);
              }
            },
            Err: (err) => setError(err),
          });
        });
      },
      Err: (err) => {
        setIsGenerating(false);

        // Handle name collision separately from other errors
        if (err.code === "NameTaken") {
          const suggestions = generateNameSuggestions(voterName, voters);
          setNameCollisionError(
            `Namnet "${voterName}" är redan taget. Förslag: ${suggestions.join(", ")}`,
          );
        } else {
          setError(err);
        }
      },
    });
  };

  const regenerateQrCode = async (voter: Voter) => {
    setIsGenerating(true);
    setQrCodeUrl(null);

    const result = await resetLogin({
      user_uuuid: voter.uuid,
    } as ResetLoginRequest);

    matchResult(result, {
      Ok: (res) => {
        const url = URL.createObjectURL(res.blob);
        setQrCodeUrl(url);
        setIsGenerating(false);
        // Refresh voter list after reset and update selectedVoter with new UUID
        VoterList({} as VoterListRequest).then((updatedVoters) => {
          matchResult(updatedVoters, {
            Ok: (response) => {
              const votersWithStatus = response.voters.map((voterData) => ({
                uuid: voterData.uuid,
                name: voterData.name,
                registeredAt: voterData.registeredAt,
                loggedIn: voterData.loggedIn,
                isHost: voterData.isHost,
              }));
              setVoters(votersWithStatus);
              // Update selectedVoter with the new UUID from backend
              const updatedVoter = votersWithStatus.find(
                (voter) => voter.name === selectedVoter?.name,
              );
              if (updatedVoter) {
                setSelectedVoter(updatedVoter);
              }
            },
            Err: (err) => setError(err),
          });
        });
      },
      Err: (err) => {
        setError(err);
        setIsGenerating(false);
      },
    });
  };

  const handleShowQrCode = (voter: Voter) => {
    setSelectedVoter(voter);
    regenerateQrCode(voter);
  };

  const handleKickOut = async (voter: Voter) => {
    if (!confirm(`Är du säker på att du vill sparka ut ${voter.name}?`)) {
      return;
    }

    setIsGenerating(true);

    const result = await resetLogin({
      user_uuuid: voter.uuid,
    } as ResetLoginRequest);

    matchResult(result, {
      Ok: (_res) => {
        // Don't display QR code for kick out - just refresh the list
        setIsGenerating(false);
        fetchVoters();
      },
      Err: (err) => {
        setError(err);
        setIsGenerating(false);
      },
    });
  };

  const handleRemoveAll = async () => {
    if (
      !confirm(
        "Är du säker på att du vill sparka ut alla deltagare? Detta kommer att logga ut alla icke-administratörer från mötet, men de kan logga in igen med nya QR-koder.",
      )
    ) {
      return;
    }

    setIsGenerating(true);

    const result = await removeAll({} as RemoveAllRequest);

    matchResult(result, {
      Ok: (_res) => {
        setIsGenerating(false);
        fetchVoters();
      },
      Err: (err) => {
        setError(err);
        setIsGenerating(false);
      },
    });
  };

  // Fuzzy filter function
  const fuzzyFilter = (
    row: any,
    columnId: string,
    value: string,
    addMeta: any,
  ) => {
    const itemRank = rankItem(row.getValue(columnId), value);
    addMeta({ itemRank });
    return itemRank.passed;
  };

  const columns = [
    columnHelper.accessor("name", {
      header: ({ column }) => (
        <button
          className="flex items-center gap-1 hover:text-[var(--color-main)] transition-colors"
          onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
        >
          Deltagarnamn
          {{
            asc: " 🔼",
            desc: " 🔽",
          }[column.getIsSorted() as string] ?? " ↕️"}
        </button>
      ),
      cell: (info) => (
        <div className="flex items-center gap-2">
          <div className="font-medium text-gray-900">{info.getValue()}</div>
          {info.row.original.isHost && (
            <span className="inline-flex px-2 py-1 text-xs font-medium rounded-full bg-purple-100 text-purple-800">
              🔐 Admin
            </span>
          )}
        </div>
      ),
      filterFn: fuzzyFilter,
      enableSorting: true,
    }),
    columnHelper.accessor("loggedIn", {
      header: ({ column }) => (
        <button
          className="flex items-center gap-1 hover:text-[var(--color-main)] transition-colors"
          onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
        >
          Status
          {{
            asc: " 🔼",
            desc: " 🔽",
          }[column.getIsSorted() as string] ?? " ↕️"}
        </button>
      ),
      cell: (info) => (
        <span
          className={`inline-flex px-3 py-1 text-sm font-medium rounded-full ${
            info.getValue()
              ? "bg-green-100 text-green-800"
              : "bg-gray-100 text-gray-600"
          }`}
        >
          {info.getValue() ? "Incheckad" : "Inte incheckad"}
        </span>
      ),
      enableSorting: true,
    }),
    columnHelper.accessor("registeredAt", {
      header: ({ column }) => (
        <button
          className="flex items-center gap-1 hover:text-[var(--color-main)] transition-colors"
          onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
        >
          Registrerad
          {{
            asc: " 🔼",
            desc: " 🔽",
          }[column.getIsSorted() as string] ?? " ↕️"}
        </button>
      ),
      cell: (info) => (
        <div className="text-sm text-gray-600">{info.getValue()}</div>
      ),
      enableSorting: true,
    }),
    columnHelper.display({
      id: "actions",
      header: "Åtgärder",
      cell: (props) => {
        const isCurrentUser = currentUserUuid === props.row.original.uuid;
        return (
          <div className="flex gap-2">
            <button
              onClick={() => handleShowQrCode(props.row.original)}
              disabled={isGenerating || isVotingActive || isCurrentUser}
              className={`px-3 py-1.5 rounded text-xs font-medium shadow-sm transition-all duration-100 ${
                isGenerating || isVotingActive || isCurrentUser
                  ? "bg-gray-300 text-gray-500 cursor-not-allowed"
                  : "bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-white hover:shadow-md active:shadow-none active:translate-y-px"
              }`}
              title={
                isCurrentUser ? "Kan inte generera QR-kod för dig själv" : ""
              }
            >
              {isCurrentUser
                ? "Ditt konto"
                : isVotingActive
                  ? "Omröstning aktiv"
                  : isGenerating
                    ? "..."
                    : "Generera QR"}
            </button>
            <button
              onClick={() => handleKickOut(props.row.original)}
              disabled={isGenerating || isVotingActive || isCurrentUser}
              className={`px-3 py-1.5 rounded text-xs font-medium shadow-sm transition-all duration-100 ${
                isGenerating || isVotingActive || isCurrentUser
                  ? "bg-gray-300 text-gray-500 cursor-not-allowed"
                  : "bg-red-500 hover:bg-red-600 text-white hover:shadow-md active:shadow-none active:translate-y-px"
              }`}
              title={isCurrentUser ? "Kan inte sparka ut dig själv" : ""}
            >
              {isCurrentUser
                ? "Du"
                : isVotingActive
                  ? "Omröstning aktiv"
                  : isGenerating
                    ? "..."
                    : "Sparka ut"}
            </button>
          </div>
        );
      },
    }),
  ];

  const table = useReactTable({
    data: voters,
    columns,
    filterFns: {
      fuzzy: fuzzyFilter,
    },
    state: {
      columnFilters,
      globalFilter,
      sorting,
      columnVisibility,
    },
    onColumnFiltersChange: setColumnFilters,
    onGlobalFilterChange: setGlobalFilter,
    onSortingChange: setSorting,
    onColumnVisibilityChange: setColumnVisibility,
    globalFilterFn: fuzzyFilter,
    getCoreRowModel: getCoreRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getSortedRowModel: getSortedRowModel(),
    debugTable: true,
  });

  const generateNameSuggestions = (
    baseName: string,
    existingVoters: Voter[],
  ): string[] => {
    const suggestions: string[] = [];
    const existingNames = existingVoters.map((v) => v.name.toLowerCase());

    // Try adding numbers
    for (let i = 2; i <= 5; i++) {
      const suggestion = `${baseName} ${i}`;
      if (!existingNames.includes(suggestion.toLowerCase())) {
        suggestions.push(suggestion);
      }
    }

    // Try adding common suffixes
    const suffixes = ["Jr", "II", "III", "(2)", "(kopia)"];
    for (const suffix of suffixes) {
      const suggestion = `${baseName} ${suffix}`;
      if (
        !existingNames.includes(suggestion.toLowerCase()) &&
        suggestions.length < 3
      ) {
        suggestions.push(suggestion);
      }
    }

    return suggestions.slice(0, 3);
  };

  const handleAddVoter = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newVoterName.trim()) return;

    // Clear any previous name collision errors
    setNameCollisionError(null);

    setSelectedVoter({
      uuid: "",
      name: newVoterName.trim(),
      registeredAt: "",
      loggedIn: false,
      isHost: isNewVoterAdmin,
    });
    generateQrCodeForNewVoter(newVoterName.trim(), isNewVoterAdmin);

    // Only clear the form if no collision error occurred
    // The form will be cleared in the success case or kept for retry in error case
  };

  const handleBack = () => {
    navigate({ to: "/meeting", search: { muuid: muid, uuuid: "" } });
  };

  if (error) {
    return <ErrorHandler error={error} />;
  }

  return (
    <div className="min-h-screen bg-[var(--color-background)] flex flex-col lg:flex-row">
      {/* Left Pane - Voter Table */}
      <div className="flex-1 lg:border-r border-gray-200 flex flex-col order-2 lg:order-1">
        {/* Header */}
        <div className="p-4 lg:p-6 border-b border-gray-200 bg-white">
          <div className="flex flex-col lg:flex-row lg:items-center justify-between mb-4 gap-4">
            <div>
              <h1 className="text-xl lg:text-2xl font-bold text-[var(--color-contours)] mb-2">
                Deltagarhantering
              </h1>
              <p className="text-sm text-gray-600">
                Hantera registrerade deltagare för mötet
              </p>
            </div>
            {isVotingActive && (
              <div className="bg-orange-50 border border-orange-200 rounded-lg p-3 lg:p-4 mb-4 lg:mb-6">
                <div className="flex items-start gap-2">
                  <span className="text-orange-600 flex-shrink-0">⚠️</span>
                  <div>
                    <h3 className="font-medium text-orange-900 text-sm lg:text-base">
                      Omröstning pågår
                    </h3>
                    <p className="text-xs lg:text-sm text-orange-800">
                      Nya deltagare kan inte bjudas in medan omröstning pågår.
                    </p>
                  </div>
                </div>
              </div>
            )}

            {/* Admin Privileges Info */}
            <div className="bg-blue-50 border border-blue-200 rounded-lg p-3 lg:p-4 mb-4 lg:mb-6 lg:block hidden">
              <div className="flex items-start gap-2">
                <span className="text-blue-600 flex-shrink-0">ℹ️</span>
                <div>
                  <h3 className="font-medium text-blue-900">
                    Administratörsbehörigheter
                  </h3>
                  <p className="text-sm text-blue-800">
                    Administratörer kan skapa omröstningar, hantera deltagare,
                    visa resultat och komma åt alla möteskontroller. Vanliga
                    deltagare kan endast rösta.
                  </p>
                </div>
              </div>
            </div>

            <div className="flex flex-wrap gap-2 lg:gap-4 mb-4 lg:mb-6">
              <button
                onClick={fetchVoters}
                className="bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-white px-3 lg:px-4 py-2 rounded shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100 text-sm lg:text-base"
              >
                Uppdatera lista
              </button>
              <button
                onClick={handleBack}
                className="bg-gray-100 hover:bg-gray-200 text-gray-700 px-3 lg:px-4 py-2 rounded shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100 text-sm lg:text-base"
              >
                Tillbaka till mötet
              </button>
              <button
                onClick={handleRemoveAll}
                disabled={isGenerating || isVotingActive}
                className={`px-3 lg:px-4 py-2 rounded shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100 text-sm lg:text-base ${
                  isGenerating || isVotingActive
                    ? "bg-gray-300 text-gray-500 cursor-not-allowed"
                    : "bg-red-500 hover:bg-red-600 text-white"
                }`}
              >
                {isGenerating ? "Sparkar ut..." : "Sparka ut alla deltagare"}
              </button>
            </div>
          </div>

          {/* Add New Voter Form */}
          <form
            onSubmit={handleAddVoter}
            className="flex flex-col lg:flex-row gap-3 items-stretch lg:items-end"
          >
            <div className="flex-1">
              <label
                htmlFor="voterName"
                className="block text-sm font-medium text-gray-700 mb-2"
              >
                Lägg till ny deltagare
              </label>
              <input
                id="voterName"
                type="text"
                value={newVoterName}
                onChange={(e) => {
                  setNewVoterName(e.target.value);
                  // Clear name collision error when user starts typing
                  if (nameCollisionError) {
                    setNameCollisionError(null);
                  }
                }}
                placeholder="Ange deltagarens fullständiga namn"
                className={`w-full p-2 lg:p-3 border rounded focus:outline-none focus:ring-2 focus:border-transparent transition-all duration-100 text-sm lg:text-base ${
                  nameCollisionError
                    ? "border-red-300 focus:ring-red-500"
                    : "border-gray-300 focus:ring-[var(--color-main)]"
                }`}
                required
              />
              {nameCollisionError && (
                <div className="mt-2 p-2 bg-red-50 border border-red-200 rounded text-sm text-red-700">
                  <div className="flex items-start">
                    <svg
                      className="w-4 h-4 text-red-500 mt-0.5 mr-2 flex-shrink-0"
                      fill="currentColor"
                      viewBox="0 0 20 20"
                    >
                      <path
                        fillRule="evenodd"
                        d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                        clipRule="evenodd"
                      />
                    </svg>
                    <div>
                      <p className="font-medium">Namn redan taget</p>
                      <p>{nameCollisionError}</p>
                    </div>
                  </div>
                </div>
              )}
              <div className="flex items-center mt-2 lg:mt-3">
                <input
                  id="isAdmin"
                  type="checkbox"
                  checked={isNewVoterAdmin}
                  onChange={(e) => setIsNewVoterAdmin(e.target.checked)}
                  className="h-4 w-4 text-[var(--color-main)] focus:ring-[var(--color-main)] border-gray-300 rounded"
                />
                <label
                  htmlFor="isAdmin"
                  className="ml-2 text-xs lg:text-sm text-gray-700"
                >
                  <span className="font-medium">Gör till admin</span>{" "}
                  <span className="text-gray-500 hidden lg:inline">
                    (full mötesåtkomst)
                  </span>
                </label>
              </div>
            </div>
            <button
              type="submit"
              disabled={isGenerating || isVotingActive}
              className={`py-2 lg:py-3 px-4 lg:px-6 rounded font-medium shadow-sm transition-all duration-100 text-sm lg:text-base whitespace-nowrap ${
                isGenerating || isVotingActive
                  ? "bg-gray-300 text-gray-500 cursor-not-allowed"
                  : "bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-white hover:shadow-md active:shadow-none active:translate-y-px"
              }`}
            >
              {isVotingActive
                ? "Omröstning aktiv"
                : isGenerating
                  ? "Genererar..."
                  : isNewVoterAdmin
                    ? "🔐 Lägg till admin"
                    : "👤 Lägg till deltagare"}
            </button>
          </form>
        </div>

        {/* Table */}
        <div className="flex-1 bg-white overflow-hidden">
          <div className="h-full flex flex-col">
            <div className="px-4 lg:px-6 py-3 lg:py-4 border-b border-gray-200 bg-gray-50">
              <div className="flex flex-col lg:flex-row lg:items-center justify-between mb-3 lg:mb-4 gap-2">
                <h2 className="text-base lg:text-lg font-semibold text-gray-900">
                  Registrerade deltagare (
                  {table.getFilteredRowModel().rows.length} av {voters.length})
                </h2>
              </div>

              {/* Search Input */}
              <div className="flex items-center gap-4">
                <div className="flex-1">
                  <input
                    value={globalFilter ?? ""}
                    onChange={(e) => setGlobalFilter(String(e.target.value))}
                    className="w-full p-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-[var(--color-main)] focus:border-transparent transition-all duration-100 text-sm lg:text-base"
                    placeholder="Sök deltagare efter namn..."
                  />
                </div>
                <div className="text-xs lg:text-sm text-gray-600 hidden lg:block">
                  {table.getFilteredRowModel().rows.length === 0 && globalFilter
                    ? "Inga deltagare hittades"
                    : ""}
                </div>
              </div>
            </div>
            <div className="flex-1 overflow-auto">
              {/* Desktop Table View */}
              <div className="hidden lg:block">
                <table className="w-full min-w-[600px]">
                  <thead className="bg-gray-50 sticky top-0 border-b border-gray-200">
                    {table.getHeaderGroups().map((headerGroup) => (
                      <tr key={headerGroup.id}>
                        {headerGroup.headers.map((header) => (
                          <th
                            key={header.id}
                            className="px-3 lg:px-6 py-3 lg:py-4 text-left text-xs lg:text-sm font-semibold text-gray-700 uppercase tracking-wider"
                          >
                            {header.isPlaceholder
                              ? null
                              : flexRender(
                                  header.column.columnDef.header,
                                  header.getContext(),
                                )}
                          </th>
                        ))}
                      </tr>
                    ))}
                  </thead>
                  <tbody className="divide-y divide-gray-200">
                    {table.getRowModel().rows.map((row) => (
                      <tr
                        key={row.original.uuid}
                        className="hover:bg-gray-50 transition-colors"
                      >
                        {row.getVisibleCells().map((cell) => (
                          <td
                            key={cell.id}
                            className="px-3 lg:px-6 py-3 lg:py-4 whitespace-nowrap text-sm lg:text-base"
                          >
                            {flexRender(
                              cell.column.columnDef.cell,
                              cell.getContext(),
                            )}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              {/* Mobile Card View */}
              <div className="lg:hidden space-y-3 p-3">
                {table.getRowModel().rows.map((row) => {
                  const voter = row.original;
                  const isCurrentUser = currentUserUuid === voter.uuid;
                  return (
                    <div
                      key={voter.uuid}
                      className="bg-white border border-gray-200 rounded-lg p-4 shadow-sm"
                    >
                      <div className="flex items-start justify-between mb-3">
                        <div className="flex-1">
                          <div className="flex items-center gap-2 mb-1">
                            <h3 className="font-medium text-gray-900 text-sm">
                              {voter.name}
                            </h3>
                            {voter.isHost && (
                              <span className="inline-flex px-2 py-0.5 text-xs font-medium rounded-full bg-purple-100 text-purple-800">
                                🔐 Admin
                              </span>
                            )}
                          </div>
                          <div className="flex flex-wrap gap-2 mb-2">
                            <span
                              className={`inline-flex px-2 py-0.5 text-xs font-medium rounded-full ${
                                voter.loggedIn
                                  ? "bg-green-100 text-green-800"
                                  : "bg-gray-100 text-gray-600"
                              }`}
                            >
                              {voter.loggedIn ? "Incheckad" : "Inte incheckad"}
                            </span>
                          </div>
                          <div className="text-xs text-gray-500">
                            Registrerad: {voter.registeredAt}
                          </div>
                        </div>
                      </div>

                      <div className="flex gap-2">
                        <button
                          onClick={() => handleShowQrCode(voter)}
                          disabled={
                            isGenerating || isVotingActive || isCurrentUser
                          }
                          className={`flex-1 py-2 px-3 rounded text-xs font-medium shadow-sm transition-all duration-100 ${
                            isGenerating || isVotingActive || isCurrentUser
                              ? "bg-gray-300 text-gray-500 cursor-not-allowed"
                              : "bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-white hover:shadow-md active:shadow-none active:translate-y-px"
                          }`}
                        >
                          {isCurrentUser
                            ? "Ditt konto"
                            : isVotingActive
                              ? "Omröstning aktiv"
                              : isGenerating
                                ? "..."
                                : "Generera QR"}
                        </button>
                        <button
                          onClick={() => handleKickOut(voter)}
                          disabled={
                            isGenerating || isVotingActive || isCurrentUser
                          }
                          className={`py-2 px-3 rounded text-xs font-medium shadow-sm transition-all duration-100 ${
                            isGenerating || isVotingActive || isCurrentUser
                              ? "bg-gray-300 text-gray-500 cursor-not-allowed"
                              : "bg-red-500 hover:bg-red-600 text-white hover:shadow-md active:shadow-none active:translate-y-px"
                          }`}
                        >
                          {isCurrentUser
                            ? "Du"
                            : isVotingActive
                              ? "Omröstning aktiv"
                              : isGenerating
                                ? "..."
                                : "Sparka"}
                        </button>
                      </div>
                    </div>
                  );
                })}

                {table.getRowModel().rows.length === 0 && (
                  <div className="text-center py-8 text-gray-500">
                    <p>Inga deltagare hittades</p>
                    {globalFilter && (
                      <p className="text-sm mt-1">
                        Prova att justera dina söktermer
                      </p>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Right Pane - QR Code */}
      <div className="w-full lg:w-80 flex flex-col bg-white border-b lg:border-b-0 lg:border-l border-gray-200 order-1 lg:order-2">
        <div className="p-4 lg:p-6 border-b border-gray-200">
          <h2 className="text-lg lg:text-xl font-semibold text-gray-900">
            QR-kodsgenerator
          </h2>
          <p className="text-sm text-gray-600 mt-1">
            Generera åtkomstkoder för deltagare
          </p>
        </div>

        <div className="flex-1 p-4 lg:p-6 flex items-center justify-center min-h-[300px] lg:min-h-0 overflow-y-auto">
          {selectedVoter ? (
            <div className="text-center space-y-4 lg:space-y-6 max-w-sm lg:max-w-md w-full">
              <div className="p-3 lg:p-4 bg-gray-50 rounded-lg">
                <p className="text-xs lg:text-sm text-gray-600 mb-1 lg:mb-2">
                  QR-kod för:
                </p>
                <p className="font-semibold text-base lg:text-lg text-gray-900 break-words">
                  {selectedVoter.name}
                </p>
              </div>

              {isGenerating ? (
                <div className="flex items-center justify-center py-8 lg:py-16">
                  <div className="text-center">
                    <div className="animate-spin rounded-full h-8 lg:h-12 w-8 lg:w-12 border-b-2 border-[var(--color-main)] mx-auto mb-3 lg:mb-4"></div>
                    <p className="text-sm lg:text-base text-gray-600">
                      Genererar QR-kod...
                    </p>
                  </div>
                </div>
              ) : qrCodeUrl ? (
                <div className="space-y-4 lg:space-y-6">
                  <div className="flex justify-center">
                    <img
                      src={qrCodeUrl}
                      alt={`QR Code for ${selectedVoter.name}`}
                      className="w-48 h-48 lg:w-64 lg:h-64 border border-gray-200 rounded-lg shadow-sm cursor-pointer hover:scale-105 transition-transform"
                      onClick={() => setIsQrModalOpen(true)}
                      title="Klicka för att förstora"
                    />
                  </div>
                  <div className="space-y-2 lg:space-y-3">
                    <p className="text-sm lg:text-base text-gray-600 px-2">
                      Skanna denna QR-kod för att gå med i mötet som{" "}
                      <span className="font-medium break-words">
                        {selectedVoter.name}
                      </span>
                    </p>
                    <button
                      onClick={() => regenerateQrCode(selectedVoter)}
                      disabled={
                        isGenerating ||
                        isVotingActive ||
                        currentUserUuid === selectedVoter.uuid
                      }
                      className={`w-full lg:w-auto px-3 lg:px-4 py-2 rounded font-medium shadow-sm transition-all duration-100 text-sm lg:text-base ${
                        isGenerating ||
                        isVotingActive ||
                        currentUserUuid === selectedVoter.uuid
                          ? "bg-gray-200 text-gray-400 cursor-not-allowed"
                          : "bg-gray-100 hover:bg-gray-200 text-gray-700 hover:shadow-md active:shadow-none active:translate-y-px"
                      }`}
                      title={
                        currentUserUuid === selectedVoter.uuid
                          ? "Kan inte regenerera din egen QR-kod"
                          : ""
                      }
                    >
                      {currentUserUuid === selectedVoter.uuid
                        ? "Din QR-kod"
                        : isVotingActive
                          ? "Omröstning aktiv"
                          : isGenerating
                            ? "Genererar..."
                            : "Regenerera QR-kod"}
                    </button>
                    <button
                      onClick={() => setIsQrModalOpen(true)}
                      disabled={!qrCodeUrl}
                      className={`px-3 lg:px-4 py-2 rounded font-medium shadow-sm transition-all duration-100 text-sm lg:text-base ${
                        !qrCodeUrl
                          ? "bg-gray-200 text-gray-400 cursor-not-allowed"
                          : "bg-blue-600 hover:bg-blue-700 text-white hover:shadow-md active:shadow-none active:translate-y-px"
                      }`}
                    >
                      🔍 Förstora
                    </button>
                  </div>
                </div>
              ) : null}
            </div>
          ) : (
            <div className="text-center text-gray-500 max-w-xs lg:max-w-sm mx-auto px-4">
              <div className="w-16 lg:w-20 h-16 lg:h-20 mx-auto mb-4 lg:mb-6 bg-gray-100 rounded-lg flex items-center justify-center">
                <svg
                  className="w-8 lg:w-10 h-8 lg:h-10 text-gray-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 4v1m6 11h2m-6 0h-2v4m0-11v3m0 0h.01M12 12h4.01M16 20h4M4 12h4m12 0h.01M5 8h2a1 1 0 001-1V6a1 1 0 00-1-1H5a1 1 0 00-1 1v1a1 1 0 001 1zm12 0h2a1 1 0 001-1V6a1 1 0 00-1-1h-2a1 1 0 00-1 1v1a1 1 0 001 1zM5 20h2a1 1 0 001-1v-1a1 1 0 00-1-1H5a1 1 0 00-1 1v1a1 1 0 001 1z"
                  />
                </svg>
              </div>
              <h3 className="text-base lg:text-lg font-medium text-gray-900 mb-2">
                Ingen deltagare vald
              </h3>
              <p className="text-sm lg:text-base text-gray-600 leading-relaxed">
                Klicka "Generera QR" för valfri deltagare för att skapa deras
                åtkomstkod
              </p>
            </div>
          )}
        </div>
      </div>

      {/* QR Code Enlargement Modal */}
      {isQrModalOpen && qrCodeUrl && selectedVoter && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-75"
          onClick={() => setIsQrModalOpen(false)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              setIsQrModalOpen(false);
            }
          }}
          tabIndex={0}
          role="dialog"
          aria-modal="true"
          aria-labelledby="qr-modal-title"
        >
          <div
            className="bg-white rounded-lg p-6 max-w-md w-full mx-4 relative"
            onClick={(e) => e.stopPropagation()}
          >
            <button
              onClick={() => setIsQrModalOpen(false)}
              className="absolute top-2 right-2 text-gray-400 hover:text-gray-600 text-2xl font-bold w-8 h-8 flex items-center justify-center rounded-full hover:bg-gray-100 transition-colors"
              aria-label="Stäng modal"
            >
              ×
            </button>
            <div className="text-center">
              <h3
                id="qr-modal-title"
                className="text-lg font-semibold text-gray-900 mb-4"
              >
                QR-kod för {selectedVoter.name}
              </h3>
              <div className="bg-white p-4 rounded-lg border border-gray-200 mb-4">
                <img
                  src={qrCodeUrl}
                  alt={`QR Code for ${selectedVoter.name}`}
                  className="w-full max-w-sm mx-auto"
                />
              </div>
              <p className="text-sm text-gray-600">
                Skanna denna kod för att logga in som {selectedVoter.name}
              </p>
              <p className="text-xs text-gray-500 mt-2">
                Tryck Escape eller klicka utanför för att stänga
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
