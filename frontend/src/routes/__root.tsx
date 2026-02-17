import { Outlet, createRootRoute } from "@tanstack/react-router";
import { Navbar } from "@/components/Navbar/Navbar";

export const Route = createRootRoute({
  component: () => (
    <>
      <Navbar />
      <div
        className="pt-18 min-h-screen"
        style={{ backgroundColor: "var(--color-background)" }}
      >
        <Outlet />
      </div>
    </>
  ),
});
