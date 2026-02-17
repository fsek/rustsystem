import { render, screen } from "@testing-library/react";
import { Input } from "./Input";
import type { Color, Size } from "../types";

const SIZES: Size[] = ["s", "sm", "m", "ml", "l", "xl"];
const COLORS: Color[] = ["primary", "secondary", "accent"];

describe("Input", () => {
  it("renders an <input> element", () => {
    const { container } = render(<Input size="m" color="primary" />);
    expect(container.querySelector("input")).toBeTruthy();
  });

  it("renders with placeholder", () => {
    render(<Input size="m" color="primary" placeholder="Enter value" />);
    expect(screen.getByPlaceholderText("Enter value")).toBeTruthy();
  });

  it("applies a border style", () => {
    const { container } = render(<Input size="m" color="primary" />);
    const input = container.querySelector("input") as HTMLInputElement;
    expect(input.style.border).toContain("solid");
  });

  it("reflects value prop", () => {
    render(<Input size="m" color="primary" value="hello" readOnly />);
    expect((screen.getByDisplayValue("hello") as HTMLInputElement).value).toBe(
      "hello",
    );
  });

  it.each(SIZES)("renders size %s without error", (size) => {
    const { container } = render(<Input size={size} color="primary" />);
    expect(container.querySelector("input")).toBeTruthy();
  });

  it.each(COLORS)("renders color %s without error", (color) => {
    const { container } = render(<Input size="m" color={color} />);
    expect(container.querySelector("input")).toBeTruthy();
  });

  it("forwards HTML input props", () => {
    const { container } = render(<Input size="m" color="primary" disabled />);
    const input = container.querySelector("input") as HTMLInputElement;
    expect(input.disabled).toBe(true);
  });

  it("passes className through", () => {
    const { container } = render(
      <Input size="m" color="primary" className="w-40" />,
    );
    expect(container.querySelector("input")?.className).toContain("w-40");
  });
});
