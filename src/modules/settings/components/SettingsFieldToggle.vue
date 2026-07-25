<script setup lang="ts">
const props = defineProps<{
  modelValue: boolean
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()

function toggle() {
  if (props.disabled) return
  emit('update:modelValue', !props.modelValue)
}
</script>

<template>
  <button
    role="switch"
    :aria-checked="modelValue"
    :disabled="disabled"
    class="settings-toggle"
    :class="{ 'settings-toggle--on': modelValue, 'settings-toggle--disabled': disabled }"
    @click="toggle"
    type="button"
  >
    <span class="settings-toggle__track" />
    <span class="settings-toggle__thumb" />
  </button>
</template>

<style scoped>
.settings-toggle {
  position: relative;
  width: 40px;
  height: 22px;
  border: none;
  border-radius: var(--radius-full);
  background: transparent;
  padding: 0;
  cursor: pointer;
  outline: none;
  transition: all var(--dur-fast) var(--ease-out);
  flex-shrink: 0;
}

.settings-toggle--disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.settings-toggle__track {
  position: absolute;
  inset: 0;
  border-radius: var(--radius-full);
  background: var(--border-default);
  transition: background var(--dur-fast) var(--ease-out);
}

.settings-toggle--on .settings-toggle__track {
  background: var(--accent);
}

.settings-toggle__thumb {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: var(--radius-full);
  background: var(--text-tertiary);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  transition:
    transform var(--dur-fast) var(--ease-spring),
    background var(--dur-fast) var(--ease-out);
}

.settings-toggle--on .settings-toggle__thumb {
  transform: translateX(18px);
  background: var(--accent-on-accent);
}

.settings-toggle:hover:not(.settings-toggle--disabled) .settings-toggle__track {
  border-color: var(--border-hover);
}

.settings-toggle:focus-visible .settings-toggle__track {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
</style>
