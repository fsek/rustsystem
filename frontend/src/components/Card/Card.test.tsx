import { render, screen } from "@testing-library/react";
import { Card } from "./Card";
import type { Color, Size } from "../types";

const SIZES: Size[] = ["s", "sm", "m", "ml", "l", "xl"];
const COLORS: Color[] = ["primary", "secondary", "accent"];

describe("Card", () => {
	it("renders a container div", () => {
		const { container } = render(<Card size="m" color="primary" />);
		expect(container.querySelector("div")).toBeTruthy();
	});

	it("renders title when provided", () => {
		render(<Card size="m" color="primary" title="My Card" />);
		expect(screen.getByText("My Card")).toBeTruthy();
	});

	it("does not render title element when omitted", () => {
		render(<Card size="m" color="primary" />);
		expect(screen.queryByText("My Card")).toBeNull();
	});

	it("renders children", () => {
		render(<Card size="m" color="primary">Card content</Card>);
		expect(screen.getByText("Card content")).toBeTruthy();
	});

	it("applies a border style", () => {
		const { container } = render(<Card size="m" color="primary" />);
		const el = container.querySelector("div") as HTMLElement;
		expect(el.style.border).toContain("solid");
	});

	it.each(SIZES)("renders size %s without error", (size) => {
		const { container } = render(<Card size={size} color="primary" />);
		expect(container.querySelector("div")).toBeTruthy();
	});

	it.each(COLORS)("renders color %s without error", (color) => {
		const { container } = render(<Card size="m" color={color} />);
		expect(container.querySelector("div")).toBeTruthy();
	});

	it("passes className through", () => {
		const { container } = render(<Card size="m" color="primary" className="w-40" />);
		expect(container.querySelector("div")?.className).toContain("w-40");
	});
});
