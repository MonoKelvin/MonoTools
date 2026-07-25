<script setup lang="ts">
import { ref, computed } from 'vue'
import MtButton from '@/ui/components/MtButton.vue'
import { Plus, X } from '@lucide/vue'

const props = defineProps<{
  modelValue: string[]
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string[]): void
}>()

const paths = computed(() => props.modelValue ?? [])
const editingIndex = ref<number | null>(null)
const editingValue = ref('')

function addPath() {
  editingIndex.value = -1 // -1 means new path mode
  editingValue.value = ''
}

function removePath(index: number) {
  const newPaths = [...paths.value]
  newPaths.splice(index, 1)
  emit('update:modelValue', newPaths)
}

function startEdit(index: number) {
  editingIndex.value = index
  editingValue.value = paths.value[index]
}

function commitEdit() {
  if (editingIndex.value === -1) {
    // Add new path
    const trimmed = editingValue.value.trim()
    if (trimmed && !paths.value.includes(trimmed)) {
      emit('update:modelValue', [...paths.value, trimmed])
    }
  } else if (editingIndex.value >= 0) {
    // Update existing path
    const newPaths = [...paths.value]
    const trimmed = editingValue.value.trim()
    if (trimmed) {
      newPaths[editingIndex.value] = trimmed
      emit('update:modelValue', newPaths)
    }
  }
  editingIndex.value = null
}

function cancelEdit() {
  editingIndex.value = null
}
</script>

<template>
  <div class="settings-path-list">
    <div v-if="paths.length === 0" class="settings-path-list__empty">
      暂无路径，点击下方按钮添加
    </div>
    <ul v-else class="settings-path-list__list">
      <li
        v-for="(path, index) in paths"
        :key="index"
        class="settings-path-list__item"
      >
        <template v-if="editingIndex === index">
          <input
            v-model="editingValue"
            class="settings-path-list__input"
            placeholder="输入路径..."
            @blur="commitEdit"
            @keydown.enter="commitEdit"
            @keydown.escape="cancelEdit"
            type="text"
          />
        </template>
        <template v-else>
          <span class="settings-path-list__path">{{ path }}</span>
          <div class="settings-path-list__actions">
            <button class="settings-path-list__action-btn" @click="startEdit(index)" type="button">
              编辑
            </button>
            <button class="settings-path-list__action-btn settings-path-list__action-btn--danger" @click="removePath(index)" type="button">
              <X :size="12" />
            </button>
          </div>
        </template>
      </li>
    </ul>
    <MtButton
      variant="ghost"
      size="sm"
      :disabled="disabled"
      @click="addPath"
      icon="Plus"
    >
      添加目录
    </MtButton>
  </div>
</template>

<style scoped>
.settings-path-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
  min-width: 280px;
}

.settings-path-list__empty {
  font-size: var(--text-sm);
  color: var(--text-quaternary);
  padding: var(--sp-3) 0;
}

.settings-path-list__list {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
}

.settings-path-list__item {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-2) var(--sp-3);
  border-radius: var(--radius-sm);
  background: var(--surface-elevated, rgba(255, 255, 255, 0.04));
  border: 1px solid var(--border-default);
}

.settings-path-list__path {
  flex: 1;
  font-size: var(--text-sm);
  font-family: var(--font-mono);
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.settings-path-list__input {
  flex: 1;
  font-size: var(--text-sm);
  font-family: var(--font-mono);
  color: var(--text-primary);
  background: transparent;
  border: none;
  outline: none;
  padding: 0;
}

.settings-path-list__actions {
  display: flex;
  gap: var(--sp-2);
  flex-shrink: 0;
}

.settings-path-list__action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 2px 6px;
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  background: transparent;
  border: none;
  border-radius: var(--radius-xs);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);

  &:hover {
    color: var(--text-primary);
    background: var(--list-hover-bg);
  }

  &.settings-path-list__action-btn--danger:hover {
    color: var(--color-danger);
    background: var(--color-danger-soft);
  }
}
</style>
