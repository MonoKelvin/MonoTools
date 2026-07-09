<script setup lang="ts">
interface Props {
  hoverable?: boolean
  clickable?: boolean
}

withDefaults(defineProps<Props>(), {
  hoverable: false,
  clickable: false,
})

const emit = defineEmits<{
  (e: 'click', event: MouseEvent): void
}>()
</script>

<template>
  <div
    :class="['mt-card', { 'mt-card--hoverable': hoverable, 'mt-card--clickable': clickable }]"
    @click="clickable && emit('click', $event)"
  >
    <slot />
  </div>
</template>

<style scoped>
.mt-card {
  background: var(--surface);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-lg);
  padding: var(--sp-5);
  transition: all var(--dur-fast) var(--ease-out);
}

.mt-card--hoverable:hover {
  border-color: var(--border-hover);
  box-shadow: var(--shadow-md);
}

.mt-card--clickable {
  cursor: pointer;
}

.mt-card--clickable:hover {
  transform: translateY(-1px);
}
</style>
