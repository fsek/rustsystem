import { useState, useEffect } from "react";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  useReactTable,
  getFilteredRowModel,
  getSortedRowModel,
  type ColumnFiltersState,
  type SortingState,
  type VisibilityState,
} from "@tanstack/react-table";
import { rankItem } from "@tanstack/match-sorter-utils";
import {
  newVoter,
  startInvite,
  type NewVoterRequest,
  type startInviteRequest,
} from "@/api/host/newVoter";
import { startInviteWait } from "@/api/host/inviteEvent";
import { matchResult } from "@/result";
import type { APIError } from "@/api/error";
import ErrorHandler from "@/components/error";
import { VoterList, type VoterListRequest } from "@/api/host/voterList";
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
  status: "active" | "pending";
}

const columnHelper = createColumnHelper<Voter>();

function RouteComponent() {
  const navigate = useNavigate();
  const search = Route.useSearch();
  const muid = search.muid;

  const [voters, setVoters] = useState<Voter[]>([]);

  const [newVoterName, setNewVoterName] = useState("");
  const [selectedVoter, setSelectedVoter] = useState<Voter | null>(null);
  const [qrCodeUrl, setQrCodeUrl] = useState<string | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<APIError | null>(null);
  const [inviteReady, setInviteReady] = useState(false);
  const [globalFilter, setGlobalFilter] = useState("");
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const [sorting, setSorting] = useState<SortingState>([]);
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({});

  const fetchVoters = async () => {
    const result = await VoterList({} as VoterListRequest);
    matchResult(result, {
      Ok: (response) => {
        const votersWithStatus = response.voters.map((voter) => ({
          uuid: voter.uuid,
          name: voter.name,
          registeredAt: new Date().toLocaleString(),
          status: "active" as const,
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
    inviteEvent.onmessage = function (event) {
      if (event.data === "Ready") {
        setInviteReady(true);
      }
    };

    inviteEvent.onopen = function () {
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

  const generateQrCode = async (voterName: string) => {
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

    const result = await newVoter({
      voterName: voterName,
      isHost: false,
    } as NewVoterRequest);

    matchResult(result, {
      Ok: (res) => {
        const url = URL.createObjectURL(res.blob);
        setQrCodeUrl(url);
        setIsGenerating(false);
      },
      Err: (err) => {
        setError(err);
        setIsGenerating(false);
      },
    });
  };

  const handleShowQrCode = (voter: Voter) => {
    setSelectedVoter(voter);
    generateQrCode(voter.name);
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
          Voter Name
          {{
            asc: " 🔼",
            desc: " 🔽",
          }[column.getIsSorted() as string] ?? " ↕️"}
        </button>
      ),
      cell: (info) => (
        <div className="font-medium text-gray-900">{info.getValue()}</div>
      ),
      filterFn: fuzzyFilter,
      enableSorting: true,
    }),
    columnHelper.accessor("status", {
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
            info.getValue() === "active"
              ? "bg-green-100 text-green-800"
              : "bg-yellow-100 text-yellow-800"
          }`}
        >
          {info.getValue()}
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
          Registered At
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
      header: "Actions",
      cell: (props) => (
        <button
          onClick={() => handleShowQrCode(props.row.original)}
          className="bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-white px-4 py-2 rounded text-sm font-medium shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100"
        >
          Generate QR
        </button>
      ),
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

  const handleAddVoter = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newVoterName.trim()) return;

    const newVoter: Voter = {
      uuid: Date.now().toString(),
      name: newVoterName.trim(),
      registeredAt: new Date().toLocaleString(),
      status: "pending",
    };

    setVoters((prev) => [...prev, newVoter]);
    setSelectedVoter(newVoter);
    generateQrCode(newVoter.name);
    setNewVoterName("");

    // Refresh voter list after adding
    setTimeout(() => fetchVoters(), 1000);
  };

  const handleBack = () => {
    navigate({ to: "/meeting", search: { muuid: muid, uuuid: "" } });
  };

  if (error) {
    return <ErrorHandler error={error} />;
  }

  return (
    <div className="h-screen bg-[var(--color-background)] flex">
      {/* Left Pane - Voter Table */}
      <div className="flex-1 border-r border-gray-200 flex flex-col">
        {/* Header */}
        <div className="p-6 border-b border-gray-200 bg-white">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h1 className="text-2xl font-bold text-[var(--color-contours)] mb-2">
                Voter Management
              </h1>
              <p className="text-sm text-gray-600">
                Manage registered voters for the meeting
              </p>
            </div>
            <div className="flex gap-2">
              <button
                onClick={fetchVoters}
                className="bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-white px-4 py-2 rounded shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100"
              >
                Refresh
              </button>
              <button
                onClick={handleBack}
                className="bg-gray-100 hover:bg-gray-200 text-gray-700 px-4 py-2 rounded shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100"
              >
                ← Back
              </button>
            </div>
          </div>

          {/* Add New Voter Form */}
          <form onSubmit={handleAddVoter} className="flex gap-3 items-end">
            <div className="flex-1">
              <label
                htmlFor="voterName"
                className="block text-sm font-medium text-gray-700 mb-2"
              >
                Add New Voter
              </label>
              <input
                id="voterName"
                type="text"
                value={newVoterName}
                onChange={(e) => setNewVoterName(e.target.value)}
                placeholder="Enter voter's full name"
                className="w-full p-3 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-[var(--color-main)] focus:border-transparent transition-all duration-100"
                required
              />
            </div>
            <button
              type="submit"
              className="bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-white py-3 px-6 rounded font-medium shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100"
            >
              Add & Generate QR
            </button>
          </form>
        </div>

        {/* Table */}
        <div className="flex-1 bg-white overflow-hidden">
          <div className="h-full flex flex-col">
            <div className="px-6 py-4 border-b border-gray-200 bg-gray-50">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold text-gray-900">
                  Registered Voters ({table.getFilteredRowModel().rows.length}{" "}
                  of {voters.length})
                </h2>
              </div>

              {/* Search Input */}
              <div className="flex items-center gap-4">
                <div className="flex-1">
                  <input
                    value={globalFilter ?? ""}
                    onChange={(e) => setGlobalFilter(String(e.target.value))}
                    className="w-full p-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-[var(--color-main)] focus:border-transparent transition-all duration-100"
                    placeholder="Search voters by name..."
                  />
                </div>
                <div className="text-sm text-gray-600">
                  {table.getFilteredRowModel().rows.length === 0 && globalFilter
                    ? "No voters found"
                    : ""}
                </div>
              </div>
            </div>
            <div className="flex-1 overflow-auto">
              <table className="w-full">
                <thead className="bg-gray-50 sticky top-0 border-b border-gray-200">
                  {table.getHeaderGroups().map((headerGroup) => (
                    <tr key={headerGroup.id}>
                      {headerGroup.headers.map((header) => (
                        <th
                          key={header.id}
                          className="px-6 py-4 text-left text-sm font-semibold text-gray-700 uppercase tracking-wider"
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
                          className="px-6 py-4 whitespace-nowrap"
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
          </div>
        </div>
      </div>

      {/* Right Pane - QR Code */}
      <div className="w-80 flex flex-col bg-white">
        <div className="p-6 border-b border-gray-200">
          <h2 className="text-xl font-semibold text-gray-900">
            QR Code Generator
          </h2>
          <p className="text-sm text-gray-600 mt-1">
            Generate access codes for voters
          </p>
        </div>

        <div className="flex-1 p-6 flex items-center justify-center">
          {selectedVoter ? (
            <div className="text-center space-y-6 max-w-md w-full">
              <div className="p-4 bg-gray-50 rounded-lg">
                <p className="text-sm text-gray-600 mb-2">QR Code for:</p>
                <p className="font-semibold text-lg text-gray-900">
                  {selectedVoter.name}
                </p>
              </div>

              {isGenerating ? (
                <div className="flex items-center justify-center py-16">
                  <div className="text-center">
                    <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-[var(--color-main)] mx-auto mb-4"></div>
                    <p className="text-gray-600">Generating QR code...</p>
                  </div>
                </div>
              ) : qrCodeUrl ? (
                <div className="space-y-6">
                  <div className="flex justify-center">
                    <img
                      src={qrCodeUrl}
                      alt={`QR Code for ${selectedVoter.name}`}
                      className="w-64 h-64 border border-gray-200 rounded-lg shadow-sm"
                    />
                  </div>
                  <div className="space-y-3">
                    <p className="text-gray-600">
                      Scan this QR code to join the meeting as{" "}
                      <span className="font-medium">{selectedVoter.name}</span>
                    </p>
                    <button
                      onClick={() => generateQrCode(selectedVoter.name)}
                      className="bg-gray-100 hover:bg-gray-200 text-gray-700 px-4 py-2 rounded shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100"
                    >
                      Regenerate QR Code
                    </button>
                  </div>
                </div>
              ) : null}
            </div>
          ) : (
            <div className="text-center text-gray-500 max-w-sm">
              <div className="w-20 h-20 mx-auto mb-6 bg-gray-100 rounded-lg flex items-center justify-center">
                <svg
                  className="w-10 h-10 text-gray-400"
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
              <h3 className="text-lg font-medium text-gray-900 mb-2">
                No voter selected
              </h3>
              <p className="text-gray-600">
                Click "Generate QR" for any voter in the table to create their
                access code
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
