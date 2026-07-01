import { ref, onMounted, onUnmounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'

const isVisible = ref(false)
const currentWindow = ref<Awaited<ReturnType<typeof getCurrentWindow>> | null>(null)

export function useWindow() {
  onMounted(async () => {
    currentWindow.value = await getCurrentWindow()
  })

  async function showWindow() {
    if (currentWindow.value) {
      await currentWindow.value.show()
      await currentWindow.value.setFocus()
      isVisible.value = true
    }
  }

  async function hideWindow() {
    if (currentWindow.value) {
      await currentWindow.value.hide()
      isVisible.value = false
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
    if (currentWindow.value) {
      await currentWindow.value.center()
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
