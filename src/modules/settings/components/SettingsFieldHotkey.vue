<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'
import MtButton from '@/ui/components/MtButton.vue'
import { Keyboard } from '@lucide/vue'

const props = defineProps<{
  modelValue: string
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

type State = 'idle' | 'recording' | 'captured'

const state = ref<State>('idle')
const displayValue = ref(props.modelValue || '未设置')

const isRecording = ref(false)
const capturedKeys: string[] = []

function startRecord() {
  if (props.disabled) return
  state.value = 'recording'
  isRecording.value = true
  capturedKeys.length = 0
}

function stopRecord() {
  isRecording.value = false
  if (capturedKeys.length > 0) {
    const hotkey = capturedKeys.join('+')
    emit('update:modelValue', hotkey)
    displayValue.value = hotkey
  }
  state.value = 'idle'
}

function onCancel() {
  stopRecord()
}

function onKeyDown(e: KeyboardEvent) {
  if (!isRecording.value) return
  e.preventDefault()

  const parts: string[] = []
  if (e.ctrlKey) parts.push('Ctrl')
  if (e.altKey) parts.push('Alt')
  if (e.shiftKey) parts.push('Shift')
  if (e.metaKey) parts.push('Meta')

  // Ignore modifier-only combinations
  if (parts.length === 0 && !['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) {
    parts.push(e.key)
  }

  capturedKeys.push(...parts)
  displayValue.value = capturedKeys.join('+')

  // Auto-stop after key capture
  setTimeout(() => stopRecord(), 100)
}

onMounted(() => {
  window.addEventListener('keydown', onKeyDown)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeyDown)
})
</script>

<template>
  <div class="settings-hotkey">
    <Keyboard :size="14" class="settings-hotkey__icon" />
    <span class="settings-hotkey__value">{{ displayValue }}</span>
    <button
      v-if="state !== 'recording'"
      class="settings-hotkey__btn"
      :disabled="disabled"
      @click="startRecord"
      type="button"
    >
      录制
    </button>
    <MtButton
      v-else
      variant="ghost"
      size="sm"
      @click="onCancel"
    >
      取消
    </MtButton>
  </div>
</template>

<style scoped>
.settings-hotkey {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-2) var(--sp-3);
  border-radius: var(--radius-sm);
  background: var(--surface-elevated, rgba(255, 255, 255, 0.04));
  border: 1px solid var(--border-default);
}

.settings-hotkey__icon {
  color: var(--text-tertiary);
  flex-shrink: 0;
}

.settings-hotkey__value {
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-primary);
  font-family: var(--font-mono);
  min-width: 60px;
}

.settings-hotkey__btn {
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-tertiary);
  background: transparent;
  border: none;
  padding: var(--sp-1) var(--sp-3);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);

  &:hover:not(:disabled) {
    color: var(--text-primary);
    background: var(--list-hover-bg);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}
</style>
