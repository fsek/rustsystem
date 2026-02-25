import { defineConfig } from "vite";
import viteReact from "@vitejs/plugin-react";
import { TanStackRouterVite } from "@tanstack/router-plugin/vite";
import { resolve } from "node:path";
import tailwindcss from "@tailwindcss/vite";

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
      "/api": "http://localhost:1443",
    },
  },
  define: {
    "import.meta.env.DEV": JSON.stringify(process.env.DEV),
    "import.meta.env.SALT_HEX": JSON.stringify(process.env.SALT_HEX),
    "import.meta.env.KEYGEN_ITERATIONS": JSON.stringify(
      process.env.KEYGEN_ITERATIONS,
    ),
    "import.meta.env.API_ENDPOINT_TRUSTAUTH": process.env.API_ENDPOINT_TRUSTAUTH
      ? JSON.stringify(process.env.API_ENDPOINT_TRUSTAUTH)
      : undefined,
  },
  build: {
    outDir: "dist",
  },
});
