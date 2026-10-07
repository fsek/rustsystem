import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import type { AgendaView } from "@/api/meeting";
import { AgendaPanel, type AgendaHostActions } from "./AgendaPanel";

const AGENDA: AgendaView = {
  points: [
    { level: 1, title: "Opening", body: "" },
    { level: 2, title: "Election of chair", body: "Nominees: Alice, Bob" },
    { level: 1, title: "Closing", body: "" },
  ],
  current: 1,
};

function hostActions(): AgendaHostActions {
  return {
    goTo: vi.fn().mockResolvedValue(undefined),
    takeAttendance: vi.fn().mockResolvedValue({
      takenAt: "2026-10-07T18:00:00Z",
      point: { index: 1, title: "Election of chair" },
      present: [
        { id: "a", name: "Alice", isHost: true },
        { id: "b", name: "Bob", isHost: false },
      ],
    }),
    loadSource: vi.fn().mockResolvedValue("# Opening\n"),
    save: vi.fn().mockResolvedValue(undefined),
    clear: vi.fn().mockResolvedValue(undefined),
  };
}

describe("AgendaPanel", () => {
  it("renders nothing for voters when there is no agenda", () => {
    const { container } = render(<AgendaPanel agenda={null} />);
    expect(container.innerHTML).toBe("");
  });

  it("shows every point and marks the current one with its body", () => {
    render(<AgendaPanel agenda={AGENDA} />);
    expect(screen.getAllByRole("listitem")).toHaveLength(3);
    const current = screen
      .getAllByRole("listitem")
      .find((li) => li.getAttribute("aria-current") === "step");
    expect(current?.textContent).toContain("Election of chair");
    expect(current?.textContent).toContain("Nominees: Alice, Bob");
  });

  it("has no host controls for voters", () => {
    render(<AgendaPanel agenda={AGENDA} />);
    expect(screen.queryByText("Next →")).toBeNull();
    expect(screen.queryByText("Take attendance")).toBeNull();
    expect(screen.queryByText("Edit")).toBeNull();
  });

  it("moves to the next and previous point", async () => {
    const host = hostActions();
    render(<AgendaPanel agenda={AGENDA} host={host} />);
    fireEvent.click(screen.getByText("Next →"));
    await waitFor(() => expect(host.goTo).toHaveBeenCalledWith(2));
    fireEvent.click(screen.getByText("← Previous"));
    await waitFor(() => expect(host.goTo).toHaveBeenCalledWith(0));
  });

  it("jumps to a point when it is clicked", async () => {
    const host = hostActions();
    render(<AgendaPanel agenda={AGENDA} host={host} />);
    fireEvent.click(screen.getByText("Closing"));
    await waitFor(() => expect(host.goTo).toHaveBeenCalledWith(2));
  });

  it("disables Previous at the first point and Next at the last", () => {
    const host = hostActions();
    const { rerender } = render(
      <AgendaPanel agenda={{ ...AGENDA, current: 0 }} host={host} />,
    );
    expect(screen.getByText("← Previous").closest("button")?.disabled).toBe(
      true,
    );
    rerender(<AgendaPanel agenda={{ ...AGENDA, current: 2 }} host={host} />);
    expect(screen.getByText("Next →").closest("button")?.disabled).toBe(true);
  });

  it("confirms how many members attendance recorded", async () => {
    render(<AgendaPanel agenda={AGENDA} host={hostActions()} />);
    fireEvent.click(screen.getByText("Take attendance"));
    expect(
      await screen.findByText(
        "Attendance recorded: 2 members at “Election of chair”.",
      ),
    ).toBeTruthy();
  });

  it("edits the agenda's Markdown and saves it", async () => {
    const host = hostActions();
    render(<AgendaPanel agenda={AGENDA} host={host} />);
    fireEvent.click(screen.getByText("Edit"));
    const editor = await screen.findByLabelText("Agenda (Markdown)");
    expect((editor as HTMLTextAreaElement).value).toBe("# Opening\n");
    fireEvent.change(editor, { target: { value: "# New\n" } });
    fireEvent.click(screen.getByText("Save agenda"));
    await waitFor(() => expect(host.save).toHaveBeenCalledWith("# New\n"));
  });

  it("offers to add an agenda to hosts when there is none", () => {
    render(<AgendaPanel agenda={null} host={hostActions()} />);
    expect(screen.getByText("Add agenda")).toBeTruthy();
    expect(screen.getByText("Take attendance")).toBeTruthy();
  });

  it("shows server errors", async () => {
    const host = hostActions();
    host.goTo = vi.fn().mockRejectedValue(new Error("Nope"));
    render(<AgendaPanel agenda={AGENDA} host={host} />);
    fireEvent.click(screen.getByText("Next →"));
    expect(await screen.findByText("Nope")).toBeTruthy();
  });
});
