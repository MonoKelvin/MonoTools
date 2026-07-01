import { definePreset } from '@primeuix/themes'
import Aura from '@primeuix/themes/aura'

/**
 * MonoTools Custom PrimeVue Theme
 * 基于 Linear Design System
 */
export const MonoToolsPreset = definePreset(Aura, {
  semantic: {
    primary: {
      50: '#eef0ff',
      100: '#d0d4ff',
      200: '#a8b0ff',
      300: '#828fff',
      400: '#5e6ad2',
      500: '#5e6ad2',
      600: '#4a54a8',
      700: '#3a4282',
      800: '#2a3060',
      900: '#1c2040',
      950: '#101224'
    },
    colorScheme: {
      dark: {
        surface: {
          0: '#010102',
          50: '#0a0b0c',
          100: '#0f1011',
          200: '#141516',
          300: '#18191a',
          400: '#191a1b',
          500: '#23252a',
          600: '#34343a',
          700: '#3e3e44',
          800: '#4a4a50',
          900: '#56565c',
          950: '#62666d'
        },
        primary: {
          color: '#5e6ad2',
          contrastColor: '#ffffff',
          hoverColor: '#828fff',
          activeColor: '#5e69d1'
        },
        text: {
          color: '#f7f8f8',
          mutedColor: '#d0d6e0',
          hoverColor: '#ffffff'
        },
        content: {
          background: '#0f1011',
          hoverBackground: '#141516',
          borderColor: '#23252a'
        },
        overlay: {
          background: '#141516',
          borderColor: '#23252a'
        },
        formField: {
          background: '#0f1011',
          borderColor: '#23252a',
          hoverBorderColor: '#34343a',
          focusBorderColor: '#5e6ad2',
          color: '#f7f8f8',
          placeholderColor: '#8a8f98'
        }
      }
    }
  },
  components: {
    button: {
      root: {
        borderRadius: '8px',
        padding: '8px 14px',
        fontFamily: 'var(--mt-font-body)',
        fontSize: '14px',
        fontWeight: '500'
      }
    },
    inputtext: {
      root: {
        background: '#0f1011',
        borderColor: '#23252a',
        hoverBorderColor: '#34343a',
        focusBorderColor: '#5e6ad2',
        color: '#f7f8f8',
        borderRadius: '8px',
        padding: '8px 12px',
        fontFamily: 'var(--mt-font-body)'
      }
    },
    card: {
      root: {
        background: '#0f1011',
        borderColor: '#23252a',
        borderRadius: '12px',
        padding: '24px'
      }
    },
    dialog: {
      root: {
        background: '#141516',
        borderColor: '#23252a',
        borderRadius: '16px',
        color: '#f7f8f8'
      }
    },
    listbox: {
      root: {
        background: '#0f1011',
        borderColor: '#23252a',
        borderRadius: '8px'
      },
      option: {
        focusBackground: '#18191a',
        selectedBackground: 'rgba(94, 106, 210, 0.15)',
        selectedColor: '#f7f8f8',
        padding: '10px 14px'
      }
    }
  }
})
