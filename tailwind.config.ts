import type { Config } from "tailwindcss";

export default {
  darkMode: ["class"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        felt: {
          900: "#0a2218",
          800: "#0d2f1f",
          700: "#103926",
        },
        gold: {
          400: "#f5c842",
          500: "#e6b800",
          600: "#c9a200",
        },
        chip: {
          red: "#e84141",
          blue: "#4169e1",
          black: "#1a1a1a",
          white: "#f5f5f5",
        },
      },
      fontFamily: {
        card: ["Georgia", "serif"],
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
} satisfies Config;
