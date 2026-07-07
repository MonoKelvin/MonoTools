import type { Config } from 'tailwindcss'

export default {
  content: ['./index.html', './src/**/*.{vue,ts,tsx,js,jsx}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // 与 theme.scss 保持一致
        bg: {
          primary: '#1a1a2e',
          secondary: '#16213e',
          tertiary: '#0f3460',
        },
        text: {
          primary: '#eaeaea',
          secondary: '#a0a0b0',
          tertiary: '#6c6c7e',
        },
        accent: {
          DEFAULT: '#ff6b6b',
          hover: '#ff8787',
        },
      },
      fontFamily: {
        sans: [
          'Source Han Sans',
          '-apple-system',
          'BlinkMacSystemFont',
          'Segoe UI',
          'PingFang SC',
          'Hiragino Sans GB',
          'Microsoft YaHei',
          'sans-serif',
        ],
        mono: ['JetBrains Mono', 'Fira Code', 'SF Mono', 'Cascadia Code', 'monospace'],
      },
      borderRadius: {
        sm: '8px',
        md: '12px',
        lg: '16px',
        xl: '20px',
      },
      boxShadow: {
        sm: '0 2px 8px rgba(0,0,0,0.3)',
        md: '0 8px 32px rgba(0,0,0,0.4)',
        lg: '0 16px 64px rgba(0,0,0,0.5)',
      },
      backdropBlur: {
        xs: '4px',
      },
    },
  },
  plugins: [],
} satisfies Config
