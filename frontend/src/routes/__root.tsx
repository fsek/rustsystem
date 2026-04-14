import { Outlet, createRootRoute } from "@tanstack/react-router";
import { createContext, useContext } from "react";
import { Navbar } from "@/components/Navbar/Navbar";
import { Footer } from "@/components/Footer/Footer";
import { fetchLimits, type Limits } from "@/api/config";

export const LimitsContext = createContext<Limits | null>(null);

export function useLimits(): Limits {
  const ctx = useContext(LimitsContext);
  if (!ctx) throw new Error("useLimits used outside of LimitsContext provider");
  return ctx;
}

export const Route = createRootRoute({
  loader: () => fetchLimits(),
  component: RootComponent,
});

function RootComponent() {
  const limits = Route.useLoaderData();
  return (
    <LimitsContext.Provider value={limits}>
      <Navbar />
      <div
        className="pt-18 min-h-screen"
        style={{ backgroundColor: "var(--pageBg)" }}
      >
        <Outlet />
      </div>
      <Footer />
    </LimitsContext.Provider>
  );
}
