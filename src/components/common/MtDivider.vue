<script setup lang="ts">
interface Props {
  vertical?: boolean
  inset?: boolean
}

withDefaults(defineProps<Props>(), {
  vertical: false,
  inset: false,
})
</script>

<template>
  <div :class="['mt-divider', { 'mt-divider--vertical': vertical, 'mt-divider--inset': inset }]"></div>
</template>

<style scoped>
.mt-divider {
  background: var(--border-subtle);
}

.mt-divider:not(.mt-divider--vertical) {
  height: 1px;
  width: 100%;
}

.mt-divider--vertical {
  width: 1px;
  height: 100%;
}

.mt-divider--inset {
  margin: 0 var(--sp-5);
}
</style>
