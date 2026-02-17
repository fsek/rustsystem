import { render, screen } from "@testing-library/react";
import { Button } from "./Button";
import type { Color, Size } from "../types";

const SIZES: Size[] = ["s", "sm", "m", "ml", "l", "xl"];
const COLORS: Color[] = ["primary", "secondary", "accent"];

describe("Button", () => {
	it("renders children", () => {
		render(<Button size="m" color="primary">Click me</Button>);
		expect(screen.getByText("Click me")).toBeTruthy();
	});

	it("renders a <button> element", () => {
		const { container } = render(<Button size="m" color="primary">Btn</Button>);
		expect(container.querySelector("button")).toBeTruthy();
	});

	it("applies filled style by default", () => {
		const { container } = render(<Button size="m" color="primary">Btn</Button>);
		const btn = container.querySelector("button") as HTMLButtonElement;
		expect(btn.style.backgroundColor).toBeTruthy();
	});

	it("applies outline style when variant is outline", () => {
		const { container } = render(
			<Button size="m" color="primary" variant="outline">Btn</Button>,
		);
		const btn = container.querySelector("button") as HTMLButtonElement;
		expect(btn.style.border).toContain("solid");
		expect(btn.style.backgroundColor).toBe("transparent");
	});

	it.each(SIZES)("renders size %s without error", (size) => {
		const { container } = render(<Button size={size} color="primary">Btn</Button>);
		expect(container.querySelector("button")).toBeTruthy();
	});

	it.each(COLORS)("renders color %s without error", (color) => {
		const { container } = render(<Button size="m" color={color}>Btn</Button>);
		expect(container.querySelector("button")).toBeTruthy();
	});

	it("forwards HTML button props", () => {
		const { container } = render(
			<Button size="m" color="primary" disabled>Btn</Button>,
		);
		const btn = container.querySelector("button") as HTMLButtonElement;
		expect(btn.disabled).toBe(true);
	});

	it("passes className through", () => {
		const { container } = render(
			<Button size="m" color="primary" className="extra-class">Btn</Button>,
		);
		expect(container.querySelector("button")?.className).toContain("extra-class");
	});
});
