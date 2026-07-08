<script setup lang="ts">
import { ChevronUp, ChevronDown, CornerDownLeft } from "@lucide/vue"
</script>

<template>
  <div class="action-bar">
    <div class="hints">
      <span class="action-bar-item">
        <span class="kbd-group">
          <span class="kbd"><ChevronUp :size="11" :stroke-width="2.5" /></span>
          <span class="kbd"><ChevronDown :size="11" :stroke-width="2.5" /></span>
        </span>
        <span class="action-label">导航</span>
      </span>
      <span class="action-bar-item">
        <span class="kbd"><CornerDownLeft :size="11" :stroke-width="2.5" /></span>
        <span class="action-label">打开</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.action-bar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 8px 12px;
  border-top: 1px solid var(--hairline);
  background: transparent;
  color: var(--text-ash);
  font-size: 11px;
  flex-shrink: 0;
}
.hints {
  display: inline-flex;
  align-items: center;
  gap: 12px;
}
.action-bar-item {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.kbd-group {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}
.action-label {
  color: var(--text-mute);
  font-size: 11px;
}
</style>
