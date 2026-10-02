import type { Config } from 'tailwindcss'
import plugin from 'tailwindcss/plugin.js'

export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        navy: {
          900: '#0A0E17',
          800: '#0F1929',
          700: '#1E293B',
        },
        cyan: {
          400: '#00C2FF',
        },
        gold: {
          400: '#FBBF24',
        }
      }
    },
  },
  plugins: [
    // Registers the `light:` variant used throughout the app for light-theme
    // overrides. Without this, every `light:*` utility class silently
    // compiles to nothing and the light theme falls back to dark colors.
    plugin(({ addVariant }) => {
      addVariant('light', ':is(.light &)')
    }),
  ],
} satisfies Config
