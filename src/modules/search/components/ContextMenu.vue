<script setup lang="ts">
import { ref, watch } from 'vue'
import type { MtMenuItem } from '@/ui/components/MtMenu.vue'

// ============================================================================
// Props — 纯 UI 配置, 无业务逻辑
// ============================================================================

interface Props {
  visible: boolean
  x: number
  y: number
  /** 预计算好的菜单项列表, 由父级提供 */
  items: MtMenuItem[]
}

const props = defineProps<Props>()

// ============================================================================
// Emits
// ============================================================================

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void
  (e: 'close'): void
  /** 用户选中菜单项时, 回传完整的 MtMenuItem, 由父级决定做什么 */
  (e: 'select', item: MtMenuItem): void
}>()

// ============================================================================
// UI 状态
// ============================================================================

const showMenu = ref(false)
const menuX = ref(0)
const menuY = ref(0)

watch(
  () => props.visible,
  (v) => {
    if (v) {
      menuX.value = props.x
      menuY.value = props.y
      showMenu.value = true
    } else {
      showMenu.value = false
    }
  },
)

function handleSelect(item: MtMenuItem) {
  emit('select', item)
  closeMenu()
}

function closeMenu() {
  emit('update:visible', false)
  emit('close')
}
</script>

<template>
  <!-- 纯渲染: items 由父级传入, 不含任何业务决策 -->
  <MtMenu
    :items="items"
    :model-value="showMenu"
    :x="menuX"
    :y="menuY"
    @update:model-value="(v) => { if (!v) closeMenu() }"
    @select="handleSelect"
  />
</template>
