/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        accent: "#7AA8FF",
        "accent-settings": "#3478F0",
      },
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          '"PingFang SC"',
          '"Hiragino Sans"',
          '"Microsoft YaHei"',
          '"Segoe UI"',
          "sans-serif",
        ],
        mono: [
          '"SF Mono"',
          "Menlo",
          "Consolas",
          '"Courier New"',
          "monospace",
        ],
      },
    },
  },
  plugins: [],
};
