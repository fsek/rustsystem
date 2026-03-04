import { Outlet, createRootRoute } from "@tanstack/react-router";
import { Navbar } from "@/components/Navbar/Navbar";
import { Footer } from "@/components/Footer/Footer";

export const Route = createRootRoute({
  component: () => (
    <>
      <Navbar />
      <div
        className="pt-18 min-h-screen"
        style={{ backgroundColor: "var(--pageBg)" }}
      >
        <Outlet />
      </div>
      <Footer />
    </>
  ),
});
