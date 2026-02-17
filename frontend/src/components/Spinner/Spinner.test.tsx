import { render } from "@testing-library/react";
import { Spinner } from "./Spinner";
import type { Color, Size } from "../types";

const SIZES: Size[] = ["s", "sm", "m", "ml", "l", "xl"];
const COLORS: Color[] = ["primary", "secondary", "accent"];

describe("Spinner", () => {
  it("renders an <svg> element", () => {
    const { container } = render(<Spinner size="m" color="primary" />);
    expect(container.querySelector("svg")).toBeTruthy();
  });

  it("has aria-label for accessibility", () => {
    const { container } = render(<Spinner size="m" color="primary" />);
    expect(container.querySelector("svg")?.getAttribute("aria-label")).toBe(
      "Loading",
    );
  });

  it("applies animate-spin class", () => {
    const { container } = render(<Spinner size="m" color="primary" />);
    expect(container.querySelector("svg")?.getAttribute("class")).toContain(
      "animate-spin",
    );
  });

  it.each(SIZES)("renders size %s without error", (size) => {
    const { container } = render(<Spinner size={size} color="primary" />);
    expect(container.querySelector("svg")).toBeTruthy();
  });

  it.each(COLORS)("renders color %s without error", (color) => {
    const { container } = render(<Spinner size="m" color={color} />);
    expect(container.querySelector("svg")).toBeTruthy();
  });

  it("passes className through", () => {
    const { container } = render(
      <Spinner size="m" color="primary" className="extra" />,
    );
    expect(container.querySelector("svg")?.getAttribute("class")).toContain(
      "extra",
    );
  });
});
