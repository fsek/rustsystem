import { render, screen, fireEvent } from "@testing-library/react";
import { ThemeButton, THEMES, applyTheme } from "./ThemeButton";

beforeEach(() => {
	sessionStorage.clear();
	// Reset CSS variables set by applyTheme between tests.
	for (const theme of THEMES) {
		for (const key of Object.keys(theme.vars)) {
			document.documentElement.style.removeProperty(key);
		}
	}
});

describe("ThemeButton", () => {
	it("renders the Theme button", () => {
		render(<ThemeButton />);
		expect(screen.getByRole("button", { name: /theme/i })).toBeTruthy();
	});

	it("dropdown is closed by default", () => {
		render(<ThemeButton />);
		expect(screen.queryByRole("listbox")).toBeNull();
	});

	it("opens the dropdown when clicked", () => {
		render(<ThemeButton />);
		fireEvent.click(screen.getByRole("button", { name: /theme/i }));
		expect(screen.getByRole("listbox")).toBeTruthy();
	});

	it("closes the dropdown when clicked again", () => {
		render(<ThemeButton />);
		const btn = screen.getByRole("button", { name: /theme/i });
		fireEvent.click(btn);
		fireEvent.click(btn);
		expect(screen.queryByRole("listbox")).toBeNull();
	});

	it("lists all themes in the dropdown", () => {
		render(<ThemeButton />);
		fireEvent.click(screen.getByRole("button", { name: /theme/i }));
		for (const theme of THEMES) {
			expect(screen.getByText(theme.name)).toBeTruthy();
		}
	});

	it("marks the first theme as selected by default", () => {
		render(<ThemeButton />);
		fireEvent.click(screen.getByRole("button", { name: /theme/i }));
		const options = screen.getAllByRole("option");
		expect(options[0].getAttribute("aria-selected")).toBe("true");
		for (const opt of options.slice(1)) {
			expect(opt.getAttribute("aria-selected")).toBe("false");
		}
	});

	it("selecting a theme closes the dropdown", () => {
		render(<ThemeButton />);
		fireEvent.click(screen.getByRole("button", { name: /theme/i }));
		fireEvent.click(screen.getByText(THEMES[1].name));
		expect(screen.queryByRole("listbox")).toBeNull();
	});

	it("selecting a theme marks it as selected", () => {
		render(<ThemeButton />);
		fireEvent.click(screen.getByRole("button", { name: /theme/i }));
		fireEvent.click(screen.getByText(THEMES[2].name));
		fireEvent.click(screen.getByRole("button", { name: /theme/i }));
		const options = screen.getAllByRole("option");
		expect(options[2].getAttribute("aria-selected")).toBe("true");
	});

	it("saves the selected theme name to sessionStorage", () => {
		render(<ThemeButton />);
		fireEvent.click(screen.getByRole("button", { name: /theme/i }));
		fireEvent.click(screen.getByText(THEMES[1].name));
		expect(sessionStorage.getItem("fsek:theme")).toBe(THEMES[1].name);
	});

	it("restores the saved theme from sessionStorage on mount", () => {
		sessionStorage.setItem("fsek:theme", THEMES[2].name);
		render(<ThemeButton />);
		fireEvent.click(screen.getByRole("button", { name: /theme/i }));
		const options = screen.getAllByRole("option");
		expect(options[2].getAttribute("aria-selected")).toBe("true");
	});

	it("applies CSS variables when a theme is selected", () => {
		render(<ThemeButton />);
		fireEvent.click(screen.getByRole("button", { name: /theme/i }));
		fireEvent.click(screen.getByText(THEMES[1].name));
		expect(document.documentElement.style.getPropertyValue("--color-primary")).toBe(
			THEMES[1].vars["--color-primary"],
		);
	});

	it("dispatches fsek:theme-change when a theme is selected", () => {
		const handler = vi.fn();
		window.addEventListener("fsek:theme-change", handler);
		render(<ThemeButton />);
		fireEvent.click(screen.getByRole("button", { name: /theme/i }));
		fireEvent.click(screen.getByText(THEMES[1].name));
		expect(handler).toHaveBeenCalled();
		window.removeEventListener("fsek:theme-change", handler);
	});

	it("closes the dropdown on click outside", () => {
		render(
			<div>
				<ThemeButton />
				<div data-testid="outside">outside</div>
			</div>,
		);
		fireEvent.click(screen.getByRole("button", { name: /theme/i }));
		expect(screen.getByRole("listbox")).toBeTruthy();
		fireEvent.mouseDown(screen.getByTestId("outside"));
		expect(screen.queryByRole("listbox")).toBeNull();
	});
});

describe("applyTheme", () => {
	it("sets all CSS variables on document.documentElement", () => {
		applyTheme(THEMES[0]);
		for (const [key, value] of Object.entries(THEMES[0].vars)) {
			expect(document.documentElement.style.getPropertyValue(key)).toBe(value);
		}
	});

	it("dispatches fsek:theme-change event", () => {
		const handler = vi.fn();
		window.addEventListener("fsek:theme-change", handler);
		applyTheme(THEMES[0]);
		expect(handler).toHaveBeenCalledOnce();
		window.removeEventListener("fsek:theme-change", handler);
	});
});
