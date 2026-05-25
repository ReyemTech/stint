import path from "node:path";
import { defineConfig } from "vitest/config";
import solid from "vite-plugin-solid";

export default defineConfig({
  plugins: [solid()],
  resolve: {
    alias: {
      "~": path.resolve(__dirname, "src"),
    },
    // SolidJS needs the "development" condition during tests so
    // vite-plugin-solid resolves the correct dev runtime exports
    // (otherwise effects/signals behave incorrectly in jsdom).
    conditions: ["development", "browser"],
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    coverage: {
      provider: "v8",
      include: ["src/**/*.{ts,tsx}"],
      exclude: ["src/main.tsx", "src/**/*.d.ts", "src/test/**"],
      reporter: ["text", "html", "json-summary", "lcov"],
    },
  },
});
