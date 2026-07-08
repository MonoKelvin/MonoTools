<script setup lang="ts">
import { ChevronUp, ChevronDown, CornerDownLeft } from "@lucide/vue"
</script>

<template>
  <div class="action-bar">
    <div class="hints">
      <span class="hint">
        <span class="hint-keys">
          <span class="kbd"><ChevronUp :size="12" :stroke-width="2.5" /></span>
          <span class="kbd"><ChevronDown :size="12" :stroke-width="2.5" /></span>
        </span>
        <span class="hint-label">导航</span>
      </span>
      <span class="hint">
        <span class="kbd"><CornerDownLeft :size="12" :stroke-width="2.5" /></span>
        <span class="hint-label">打开</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.action-bar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: var(--sp-3) var(--sp-6);
  border-top: 1px solid var(--border-subtle);
  background: var(--surface-raised);
  flex-shrink: 0;
}

.hints {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-5);
}

.hint {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-3);
}

.hint-keys {
  display: inline-flex;
  align-items: center;
  gap: 3px;
}

.hint-label {
  color: var(--text-quaternary);
  font-size: var(--text-sm);
  font-weight: 400;
  letter-spacing: 0.01em;
}
</style>