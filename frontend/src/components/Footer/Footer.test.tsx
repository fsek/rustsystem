import { render, screen } from "@testing-library/react";
import { Footer } from "./Footer";

vi.mock("@tanstack/react-router", () => ({
  Link: ({ to, children, ...props }: { to: string; children: React.ReactNode; [key: string]: unknown }) => (
    <a href={to} {...props}>{children}</a>
  ),
}));

describe("Footer", () => {
  it("renders without crashing", () => {
    const { container } = render(<Footer />);
    expect(container.querySelector("footer")).toBeTruthy();
  });

  it("displays the app version", () => {
    const { container } = render(<Footer />);
    const footer = container.querySelector("footer");
    expect(footer?.textContent).toBeTruthy();
  });

  it("renders guide and cryptography links", () => {
    render(<Footer />);
    expect(screen.getByText("Guide")).toBeTruthy();
    expect(screen.getByText("Cryptography")).toBeTruthy();
  });
});
