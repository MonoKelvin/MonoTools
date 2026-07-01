import { onUnmounted } from 'vue'

interface HotkeyHandler {
  (event: Event): void
}

const hotkeyMap = new Map<string, HotkeyHandler>()

export function useHotkey() {
  // 全局热键注册（通过 Tauri）
  async function registerGlobal(hotkey: string, handler: HotkeyHandler) {
    try {
      // 这里需要后端支持
      // TODO: 实现全局热键注册
      hotkeyMap.set(hotkey, handler)
    } catch (error) {
      console.error('Failed to register global hotkey:', hotkey, error)
    }
  }

  async function unregisterGlobal(hotkey: string) {
    hotkeyMap.delete(hotkey)
    // TODO: 通知后端注销热键
  }

  // 本地热键监听（仅窗口激活时）
  function registerLocal(hotkey: string, handler: HotkeyHandler) {
    const parts = hotkey.toUpperCase().split('+')
    const key = parts[parts.length - 1]
    const modifiers = parts.slice(0, -1)

    const listener = (event: KeyboardEvent) => {
      if (event.key.toUpperCase() === key && modifiers.every(mod => checkModifier(mod, event))) {
        event.preventDefault()
        handler(event)
      }
    }

    window.addEventListener('keydown', listener)
    return () => window.removeEventListener('keydown', listener)
  }

  function unregisterLocal(listener: () => void) {
    listener()
  }

  function checkModifier(modifier: string, event: KeyboardEvent): boolean {
    switch (modifier) {
      case 'ALT':
        return event.altKey
      case 'CTRL':
        return event.ctrlKey
      case 'SHIFT':
        return event.shiftKey
      case 'META':
      case 'CMD':
        return event.metaKey
      default:
        return false
    }
  }

  onUnmounted(() => {
    hotkeyMap.clear()
  })

  return {
    registerGlobal,
    unregisterGlobal,
    registerLocal,
    unregisterLocal
  }
}
