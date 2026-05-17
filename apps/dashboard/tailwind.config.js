/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./src/**/*.{js,ts,jsx,tsx,mdx}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        bg: {
          DEFAULT: 'var(--color-bg)',
          primary: 'var(--bg-primary)',
          secondary: 'var(--bg-secondary)',
          card: 'var(--bg-card)',
        },
        surface: {
          DEFAULT: 'var(--color-surface)',
          elevated: 'var(--color-surface-elevated)',
        },
        primary: {
          DEFAULT: 'var(--color-primary)',
          secondary: 'var(--color-secondary)',
        },
        brand: {
          200: 'var(--brand-200)',
          400: 'var(--brand-400)',
          500: 'var(--brand-500)',
          600: 'var(--brand-600)',
          700: 'var(--brand-700)',
        },
        neon: {
          blue: 'var(--color-neon-blue)',
          green: 'var(--color-neon-green)',
        },
        semantic: {
          warning: 'var(--color-warning)',
          error: 'var(--color-error)',
        },
        text: {
          primary: 'var(--text-primary)',
          secondary: 'var(--text-secondary)',
        },
        border: {
          DEFAULT: 'var(--border)',
          subtle: 'var(--color-border-subtle)',
        },
        glow: 'var(--color-glow)',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
      },
      fontSize: {
        display: ['42px', { lineHeight: '1.1', fontWeight: '700' }],
        h1: ['32px', { lineHeight: '1.2', fontWeight: '700' }],
        h2: ['24px', { lineHeight: '1.3', fontWeight: '700' }],
        h3: ['18px', { lineHeight: '1.4', fontWeight: '600' }],
        body: ['14px', { lineHeight: '1.6', fontWeight: '400' }],
        small: ['12px', { lineHeight: '1.5', fontWeight: '400' }],
      },
      borderRadius: {
        sm: '8px',
        md: '12px',
        lg: '16px',
        xl: '24px',
        neon: '999px',
      },
      boxShadow: {
        'card-glow': '0 0 30px rgba(99, 102, 241, 0.12)',
        'card-depth': '0 8px 40px rgba(0, 0, 0, 0.45)',
        'neon': '0 0 20px rgba(99, 102, 241, 0.4)',
        'neon-blue': '0 0 20px rgba(59, 130, 246, 0.4)',
        'neon-green': '0 0 20px rgba(34, 197, 94, 0.4)',
        'glow-border': '0 0 12px rgba(99, 102, 241, 0.25)',
        'focus-ring': '0 0 0 2px var(--color-bg), 0 0 0 4px var(--color-primary)',
      },
      spacing: {
        '1': '4px',
        '2': '8px',
        '3': '12px',
        '4': '16px',
        '5': '20px',
        '6': '24px',
        '8': '32px',
        '10': '40px',
        '12': '48px',
        '16': '64px',
      },
      transitionTimingFunction: {
        base: 'ease',
      },
      transitionDuration: {
        base: '250ms',
      },
    },
  },
  plugins: [],
};
