import { render, screen } from "@testing-library/react";
import { Badge } from "./Badge";
import type { Color, Size } from "../types";

const SIZES: Size[] = ["s", "sm", "m", "ml", "l", "xl"];
const COLORS: Color[] = ["primary", "secondary", "accent"];

describe("Badge", () => {
	it("renders children", () => {
		render(<Badge size="m" color="primary">Active</Badge>);
		expect(screen.getByText("Active")).toBeTruthy();
	});

	it("renders a <span> element", () => {
		const { container } = render(<Badge size="m" color="primary">Tag</Badge>);
		expect(container.querySelector("span")).toBeTruthy();
	});

	it("applies a background color style", () => {
		const { container } = render(<Badge size="m" color="primary">Tag</Badge>);
		const span = container.querySelector("span") as HTMLSpanElement;
		expect(span.style.backgroundColor).toBeTruthy();
	});

	it.each(SIZES)("renders size %s without error", (size) => {
		const { container } = render(<Badge size={size} color="primary">Tag</Badge>);
		expect(container.querySelector("span")).toBeTruthy();
	});

	it.each(COLORS)("renders color %s without error", (color) => {
		const { container } = render(<Badge size="m" color={color}>Tag</Badge>);
		expect(container.querySelector("span")).toBeTruthy();
	});
});
