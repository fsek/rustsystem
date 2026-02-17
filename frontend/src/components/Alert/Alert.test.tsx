import { render, screen } from "@testing-library/react";
import { Alert } from "./Alert";
import type { Color, Size } from "../types";

const SIZES: Size[] = ["s", "sm", "m", "ml", "l", "xl"];
const COLORS: Color[] = ["primary", "secondary", "accent"];

describe("Alert", () => {
  it("renders children", () => {
    render(
      <Alert size="m" color="primary">
        Something went wrong
      </Alert>,
    );
    expect(screen.getByText("Something went wrong")).toBeTruthy();
  });

  it("has role alert", () => {
    render(
      <Alert size="m" color="primary">
        Message
      </Alert>,
    );
    expect(screen.getByRole("alert")).toBeTruthy();
  });

  it("applies a left border style", () => {
    const { container } = render(
      <Alert size="m" color="primary">
        Message
      </Alert>,
    );
    const el = container.querySelector("[role=alert]") as HTMLElement;
    expect(el.style.borderLeft).toContain("solid");
  });

  it("renders the info icon", () => {
    const { container } = render(
      <Alert size="m" color="primary">
        Message
      </Alert>,
    );
    expect(container.textContent).toContain("ℹ");
  });

  it.each(SIZES)("renders size %s without error", (size) => {
    render(
      <Alert size={size} color="primary">
        Message
      </Alert>,
    );
    expect(screen.getByRole("alert")).toBeTruthy();
  });

  it.each(COLORS)("renders color %s without error", (color) => {
    render(
      <Alert size="m" color={color}>
        Message
      </Alert>,
    );
    expect(screen.getByRole("alert")).toBeTruthy();
  });

  it("passes className through", () => {
    const { container } = render(
      <Alert size="m" color="primary" className="w-64">
        Message
      </Alert>,
    );
    expect(container.querySelector("[role=alert]")?.className).toContain(
      "w-64",
    );
  });
});
