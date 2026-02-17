import { render, screen, fireEvent } from "@testing-library/react";
import { VoteOption } from "./VoteOption";
import type { Color, Size } from "../types";

const SIZES: Size[] = ["s", "sm", "m", "ml", "l", "xl"];
const COLORS: Color[] = ["primary", "secondary", "accent"];

describe("VoteOption", () => {
	it("renders the label", () => {
		render(<VoteOption size="m" color="primary" label="Option A" />);
		expect(screen.getByText("Option A")).toBeTruthy();
	});

	it("renders a <button> element", () => {
		const { container } = render(<VoteOption size="m" color="primary" label="Option A" />);
		expect(container.querySelector("button")).toBeTruthy();
	});

	it("has aria-pressed=false when not selected", () => {
		const { container } = render(<VoteOption size="m" color="primary" label="A" />);
		expect(container.querySelector("button")?.getAttribute("aria-pressed")).toBe("false");
	});

	it("has aria-pressed=true when selected", () => {
		const { container } = render(<VoteOption size="m" color="primary" label="A" selected />);
		expect(container.querySelector("button")?.getAttribute("aria-pressed")).toBe("true");
	});

	it("shows checkmark svg when selected", () => {
		const { container } = render(<VoteOption size="m" color="primary" label="A" selected />);
		expect(container.querySelector("svg")).toBeTruthy();
	});

	it("does not show checkmark svg when not selected", () => {
		const { container } = render(<VoteOption size="m" color="primary" label="A" />);
		expect(container.querySelector("svg")).toBeNull();
	});

	it("calls onClick when clicked", () => {
		const onClick = vi.fn();
		render(<VoteOption size="m" color="primary" label="A" onClick={onClick} />);
		fireEvent.click(screen.getByRole("button"));
		expect(onClick).toHaveBeenCalledOnce();
	});

	it.each(SIZES)("renders size %s without error", (size) => {
		const { container } = render(<VoteOption size={size} color="primary" label="A" />);
		expect(container.querySelector("button")).toBeTruthy();
	});

	it.each(COLORS)("renders color %s without error", (color) => {
		const { container } = render(<VoteOption size="m" color={color} label="A" />);
		expect(container.querySelector("button")).toBeTruthy();
	});

	it("passes className through", () => {
		const { container } = render(
			<VoteOption size="m" color="primary" label="A" className="w-40" />,
		);
		expect(container.querySelector("button")?.className).toContain("w-40");
	});
});
