import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./app/**/*.{ts,tsx}", "./components/**/*.{ts,tsx}", "./lib/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        satBlack: "#0b0f14",
        satBlue: "#1d4ed8",
        satRed: "#dc2626",
        satYellow: "#eab308",
        satGreen: "#16a34a",
      },
      fontFamily: {
        brand: ["var(--font-brand)"],
        body: ["var(--font-body)"],
      },
      boxShadow: {
        panel: "0 18px 50px rgba(11, 15, 20, 0.08)",
      },
    },
  },
  plugins: [],
};

export default config;
