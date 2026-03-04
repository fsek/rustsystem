import { defineConfig } from "vite";
import viteReact from "@vitejs/plugin-react";
import { TanStackRouterVite } from "@tanstack/router-plugin/vite";
import { resolve } from "node:path";
import tailwindcss from "@tailwindcss/vite";
import { execSync } from "node:child_process";

const APP_VERSION = (() => {
  try {
    return execSync("git describe --tags --abbrev=0", { encoding: "utf8" }).trim();
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
    proxy: {
      // Route /api/trustauth to trustauth's /api prefix (must come before /api).
      "/api/trustauth": {
        target: "http://localhost:2443",
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api\/trustauth/, "/api"),
      },
      "/api": "http://localhost:1443",
    },
  },
  define: {
    "import.meta.env.APP_VERSION": JSON.stringify(APP_VERSION),
    "import.meta.env.DEV": JSON.stringify(process.env.DEV),
    "import.meta.env.SALT_HEX": JSON.stringify(process.env.SALT_HEX),
    "import.meta.env.KEYGEN_ITERATIONS": JSON.stringify(
      process.env.KEYGEN_ITERATIONS,
    ),
    // In dev, default to the Vite proxy path so browsers can reach trustauth
    // without CORS issues. Override with an absolute URL in production.
    "import.meta.env.API_ENDPOINT_TRUSTAUTH": JSON.stringify(
      process.env.API_ENDPOINT_TRUSTAUTH ?? "/api/trustauth",
    ),
  },
  build: {
    outDir: "dist",
  },
});
