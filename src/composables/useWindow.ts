import { ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'

const isVisible = ref(false)

export function useWindow() {
  async function showWindow() {
    try {
      const currentWindow = await getCurrentWindow()
      await currentWindow.show()
      await currentWindow.setFocus()
      isVisible.value = true
    } catch (error) {
      console.error('Failed to show window:', error)
    }
  }

  async function hideWindow() {
    try {
      const currentWindow = await getCurrentWindow()
      await currentWindow.hide()
      isVisible.value = false
    } catch (error) {
      console.error('Failed to hide window:', error)
    }
  }

  async function toggleWindow() {
    if (isVisible.value) {
      await hideWindow()
    } else {
      await showWindow()
    }
  }

  async function centerWindow() {
    try {
      const currentWindow = await getCurrentWindow()
      await currentWindow.center()
    } catch (error) {
      console.error('Failed to center window:', error)
    }
  }

  return {
    isVisible,
    showWindow,
    hideWindow,
    toggleWindow,
    centerWindow
  }
}
