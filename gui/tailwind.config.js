/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Neural network-inspired dark theme from PRD
        background: {
          deep: '#0d1b2a',
          mid: '#1b263b',
          surface: '#253346',
        },
        accent: {
          teal: '#00f5d4',      // Orchestrator
          magenta: '#f72585',   // Planner
          amber: '#ffc300',     // Workers
        },
      },
      fontFamily: {
        sans: ['IBM Plex Sans', 'system-ui', 'sans-serif'],
        mono: ['Fira Code', 'monospace'],
      },
    },
  },
  plugins: [],
}
