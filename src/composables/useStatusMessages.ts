import { computed, ref } from 'vue'
import { ACTION_BAR_TIMEOUTS } from '@/config'

export interface StatusMessage {
  id: string
  type: 'info' | 'success' | 'error' | 'loading' | 'building'
  text: string
  priority: number
  durationMs?: number
  createdAt: number
}

const messages = ref<StatusMessage[]>([])
let nextId = 1

export function useStatusMessages() {
  const currentMessage = computed(() => {
    if (messages.value.length === 0) return null
    return messages.value.reduce((a, b) => (a.priority >= b.priority ? a : b))
  })

  const hasMessages = computed(() => messages.value.length > 0)

  function addMessage(msg: Omit<StatusMessage, 'id' | 'createdAt'>) {
    const id = `msg-${nextId++}`
    const fullMsg: StatusMessage = {
      id,
      createdAt: Date.now(),
      ...msg,
    }
    messages.value.push(fullMsg)

    const duration = msg.durationMs ?? ACTION_BAR_TIMEOUTS.completedMs
    setTimeout(() => {
      removeMessage(id)
    }, duration)

    return id
  }

  function removeMessage(id: string) {
    const idx = messages.value.findIndex((m) => m.id === id)
    if (idx >= 0) {
      messages.value.splice(idx, 1)
    }
  }

  function clearMessages() {
    messages.value = []
  }

  return {
    messages,
    currentMessage,
    hasMessages,
    addMessage,
    removeMessage,
    clearMessages,
  }
}
