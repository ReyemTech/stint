import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ["-apple-system", "BlinkMacSystemFont", "SF Pro Text", "sans-serif"],
        mono: ["SF Mono", "Menlo", "monospace"],
      },
    },
  },
} satisfies Config;
