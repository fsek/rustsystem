import { createRef } from "react";
import { render, screen, fireEvent } from "@testing-library/react";
import { VoteSection } from "./VoteSection";
import type { VoteSectionHandle } from "./VoteSection";
import type { Color, Size } from "../types";

const SIZES: Size[] = ["s", "sm", "m", "ml", "l", "xl"];
const COLORS: Color[] = ["primary", "secondary", "accent"];
const OPTIONS = ["In favor", "Against", "Abstain"];

describe("VoteSection", () => {
	it("renders all options", () => {
		render(<VoteSection size="m" color="primary" options={OPTIONS} />);
		for (const opt of OPTIONS) {
			expect(screen.getByText(opt)).toBeTruthy();
		}
	});

	it("renders a button for each option", () => {
		const { container } = render(<VoteSection size="m" color="primary" options={OPTIONS} />);
		expect(container.querySelectorAll("button").length).toBe(OPTIONS.length);
	});

	it("starts with nothing selected", () => {
		const ref = createRef<VoteSectionHandle>();
		render(<VoteSection ref={ref} size="m" color="primary" options={OPTIONS} />);
		expect(ref.current?.getSelected()).toEqual([]);
	});

	it("selects an option when clicked", () => {
		const ref = createRef<VoteSectionHandle>();
		render(<VoteSection ref={ref} size="m" color="primary" options={OPTIONS} />);
		fireEvent.click(screen.getByText("In favor"));
		expect(ref.current?.getSelected()).toContain("In favor");
	});

	it("deselects an option when clicked again", () => {
		const ref = createRef<VoteSectionHandle>();
		render(<VoteSection ref={ref} size="m" color="primary" options={OPTIONS} />);
		fireEvent.click(screen.getByText("In favor"));
		fireEvent.click(screen.getByText("In favor"));
		expect(ref.current?.getSelected()).not.toContain("In favor");
	});

	it("allows selecting multiple options", () => {
		const ref = createRef<VoteSectionHandle>();
		render(<VoteSection ref={ref} size="m" color="primary" options={OPTIONS} />);
		fireEvent.click(screen.getByText("In favor"));
		fireEvent.click(screen.getByText("Abstain"));
		const selected = ref.current?.getSelected() ?? [];
		expect(selected).toContain("In favor");
		expect(selected).toContain("Abstain");
		expect(selected).not.toContain("Against");
	});

	it("getSelected reflects current state after multiple toggles", () => {
		const ref = createRef<VoteSectionHandle>();
		render(<VoteSection ref={ref} size="m" color="primary" options={OPTIONS} />);
		fireEvent.click(screen.getByText("Against"));
		fireEvent.click(screen.getByText("Abstain"));
		fireEvent.click(screen.getByText("Against")); // deselect
		expect(ref.current?.getSelected()).toEqual(["Abstain"]);
	});

	it("renders with an empty options list", () => {
		const { container } = render(<VoteSection size="m" color="primary" options={[]} />);
		expect(container.querySelectorAll("button").length).toBe(0);
	});

	it.each(SIZES)("renders size %s without error", (size) => {
		const { container } = render(<VoteSection size={size} color="primary" options={OPTIONS} />);
		expect(container.querySelectorAll("button").length).toBe(OPTIONS.length);
	});

	it.each(COLORS)("renders color %s without error", (color) => {
		const { container } = render(<VoteSection size="m" color={color} options={OPTIONS} />);
		expect(container.querySelectorAll("button").length).toBe(OPTIONS.length);
	});

	it("passes className through", () => {
		const { container } = render(
			<VoteSection size="m" color="primary" options={OPTIONS} className="custom-class" />,
		);
		expect(container.querySelector("div")?.className).toContain("custom-class");
	});
});
