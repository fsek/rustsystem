import { defineConfig } from "vite";
import viteReact from "@vitejs/plugin-react";
import { TanStackRouterVite } from "@tanstack/router-plugin/vite";
import { resolve } from "node:path";
import tailwindcss from "@tailwindcss/vite";
import { execSync } from "node:child_process";

const APP_VERSION = (() => {
  try {
    return execSync("git describe --tags --abbrev=0", {
      encoding: "utf8",
    }).trim();
  } catch {
    return "dev";
  }
})();

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    TanStackRouterVite({ autoCodeSplitting: true }),
    viteReact(),
    tailwindcss(),
  ],
  base: process.env.NODE_ENV === "production" ? "/" : "/",
  test: {
    globals: true,
    environment: "jsdom",
    environmentOptions: {
      jsdom: { url: "http://localhost/" },
    },
    include: ["src/**/*.{test,spec}.{ts,tsx}", "src/**/*-test.{ts,tsx}"],
  },
  resolve: {
    alias: {
      "@": resolve(__dirname, "./src"),
    },
  },
  server: {
    // Matches SERVER_PUBLIC_URLS in ../.env, which lets this origin call trustauth.
    port: 3000,
    strictPort: true,
    // Only the server is proxied: the browser calls trustauth directly, at the URL the server
    // announces in GET /api/config, exactly as in production.
    proxy: {
      "/api": "http://localhost:1443",
    },
  },
  define: {
    "import.meta.env.APP_VERSION": JSON.stringify(APP_VERSION),
    "import.meta.env.DEV": JSON.stringify(process.env.DEV),
  },
  build: {
    outDir: "dist",
  },
});
