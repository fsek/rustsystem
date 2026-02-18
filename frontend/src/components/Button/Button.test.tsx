import { render, screen } from "@testing-library/react";
import { Button } from "./Button";
import type { ButtonColor, Size } from "../types";

const SIZES: Size[] = ["s", "sm", "m", "ml", "l", "xl"];
const COLORS: ButtonColor[] = [
  "buttonPrimary",
  "buttonSecondary",
  "linearGrad",
  "radialGrad",
];

describe("Button", () => {
  it("renders children", () => {
    render(
      <Button size="m" color="buttonPrimary">
        Click me
      </Button>,
    );
    expect(screen.getByText("Click me")).toBeTruthy();
  });

  it("renders a <button> element", () => {
    const { container } = render(
      <Button size="m" color="buttonPrimary">
        Btn
      </Button>,
    );
    expect(container.querySelector("button")).toBeTruthy();
  });

  it("applies filled style by default", () => {
    const { container } = render(
      <Button size="m" color="buttonPrimary">
        Btn
      </Button>,
    );
    const btn = container.querySelector("button") as HTMLButtonElement;
    expect(btn.style.backgroundColor).toBeTruthy();
  });

  it("applies outline style when variant is outline", () => {
    const { container } = render(
      <Button size="m" color="buttonPrimary" variant="outline">
        Btn
      </Button>,
    );
    const btn = container.querySelector("button") as HTMLButtonElement;
    expect(btn.style.border).toContain("solid");
    expect(btn.style.backgroundColor).toBe("transparent");
  });

  it.each(SIZES)("renders size %s without error", (size) => {
    const { container } = render(
      <Button size={size} color="buttonPrimary">
        Btn
      </Button>,
    );
    expect(container.querySelector("button")).toBeTruthy();
  });

  it.each(COLORS)("renders color %s without error", (color) => {
    const { container } = render(
      <Button size="m" color={color}>
        Btn
      </Button>,
    );
    expect(container.querySelector("button")).toBeTruthy();
  });

  it("forwards HTML button props", () => {
    const { container } = render(
      <Button size="m" color="buttonPrimary" disabled>
        Btn
      </Button>,
    );
    const btn = container.querySelector("button") as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  it("passes className through", () => {
    const { container } = render(
      <Button size="m" color="buttonPrimary" className="extra-class">
        Btn
      </Button>,
    );
    expect(container.querySelector("button")?.className).toContain(
      "extra-class",
    );
  });
});
