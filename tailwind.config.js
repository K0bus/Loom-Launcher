/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        background: '#09090b',
        surface: {
          DEFAULT: '#18181b',
          muted: '#121215',
          elevated: '#27272a',
        },
        brand: {
          gold: '#F59E0B',
          'gold-hover': '#fbbf24',
          'gold-dim': '#78350f',
          50: '#fffbeb',
          100: '#fef3c7',
          200: '#fde68a',
          300: '#fcd34d',
          400: '#fbbf24',
          500: '#f59e0b',
          600: '#d97706',
          700: '#b45309',
        }
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        cinzel: ['Cinzel', 'serif'],
        mono: ['"JetBrains Mono"', 'monospace'],
      },
      spacing: {
        4.5: '1.125rem',
      },
      boxShadow: {
        'glow-gold': '0 0 24px rgba(245, 158, 11, 0.35)',
        'glow-gold-lg': '0 0 32px rgba(245, 158, 11, 0.5)',
        'glow-rose': '0 0 24px rgba(225, 29, 72, 0.35)',
      }
    },
  },
  plugins: [],
}
