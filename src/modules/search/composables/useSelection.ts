import { ref, watch, type Ref } from 'vue'
import type { SearchResult } from '@/modules/search'

export interface UseSelectionDeps {
    /** 实际可见的扁平列表 (来自 useSearchResults). */
    displayList: Ref<SearchResult[]>
    /** 可见列表长度上限 (来自 useSearchResults). */
    displayMax: Ref<number>
}

/**
 * 键盘导航与选择 composable.
 * 负责 selectedIndex, selectedGlobalId, 多选模式, 上下方向键导航.
 */
export function useSelection(deps: UseSelectionDeps) {
    const { displayList, displayMax } = deps

    const selectedIndex = ref(0)
    /**
     * 选中项的全局 ID 锚点 —— 搜索 / 折叠 / 分类变化时,
     * 用 ID 而不是 index 来追踪"用户选中的是哪个"，
     * 避免结果列表重排后 selectedIndex 指向错误位置.
     */
    const selectedGlobalId = ref<string | null>(null)

    /**
     * 多选模式下选中的项目ID集合.
     */
    const selectedIds = ref<Set<string>>(new Set())
    /**
     * 上一次选中的索引, 用于 Shift 范围选择.
     */
    const lastSelectedIndex = ref(0)

    /**
     * 当 displayList 变化时 (搜索 / 索引刷新 / 折叠切换 / 分类筛选),
     * 仅做边界 clamp, 不按 ID 重定位.
     *
     * 设计说明: 同一 SearchResult 可能出现在多个分组中 (pinned / recent / apps),
     * 每个分组中的项都是独立的. 若按 ID 追踪会导致选中跳转到第一个匹配的分组,
     * 违反"列表项独立"的交互原则.
     *
     * `flush: 'sync'` 同步触发, 避免同一 tick 内读取越界的 selectedIndex.
     */
    watch(
        displayMax,
        () => {
            if (displayMax.value === 0) {
                selectedIndex.value = 0
                selectedGlobalId.value = null
                return
            }
            if (selectedIndex.value > displayMax.value - 1) {
                selectedIndex.value = displayMax.value - 1
            } else if (selectedIndex.value < 0) {
                selectedIndex.value = 0
            }
        },
        { flush: 'sync' },
    )

    function selectNext() {
        if (selectedIndex.value < displayMax.value - 1) {
            selectedIndex.value++
            const item = displayList.value[selectedIndex.value]
            selectedGlobalId.value = item?.id ?? null
        }
    }

    function selectPrev() {
        if (selectedIndex.value > 0) {
            selectedIndex.value--
            const item = displayList.value[selectedIndex.value]
            selectedGlobalId.value = item?.id ?? null
        }
    }

    /**
     * 直接按 index 选中, 同时设置 ID 锚点. 给单击 / 双击 / hover 场景用.
     * 边界保护: 越界时 clamp 到 [0, max-1].
     */
    function selectByIndex(idx: number) {
        if (displayMax.value === 0) {
            selectedIndex.value = 0
            selectedGlobalId.value = null
            return
        }
        const clamped = Math.max(0, Math.min(idx, displayMax.value - 1))
        selectedIndex.value = clamped
        selectedGlobalId.value = displayList.value[clamped]?.id ?? null
    }

    /**
     * 多选模式下的选择方法.
     */
    function selectWithModifiers(idx: number, ctrl: boolean, shift: boolean) {
        if (displayMax.value === 0) {
            selectedIndex.value = 0
            selectedGlobalId.value = null
            return
        }
        const clamped = Math.max(0, Math.min(idx, displayMax.value - 1))
        selectedIndex.value = clamped
        const item = displayList.value[clamped]
        selectedGlobalId.value = item?.id ?? null

        if (ctrl && shift) {
            selectRangeToggle(lastSelectedIndex.value, clamped)
        } else if (shift) {
            selectRange(lastSelectedIndex.value, clamped)
        } else if (ctrl) {
            toggleSelect(clamped)
        } else {
            clearSelection()
            addToSelection(clamped)
        }

        lastSelectedIndex.value = clamped
    }

    function selectRange(from: number, to: number) {
        clearSelection()
        const start = Math.min(from, to)
        const end = Math.max(from, to)
        for (let i = start; i <= end; i++) {
            addToSelection(i)
        }
    }

    function selectRangeToggle(from: number, to: number) {
        const start = Math.min(from, to)
        const end = Math.max(from, to)
        for (let i = start; i <= end; i++) {
            toggleSelect(i)
        }
    }

    function toggleSelect(idx: number) {
        if (idx < 0 || idx >= displayMax.value) return
        const item = displayList.value[idx]
        if (!item) return
        const ids = selectedIds.value
        if (ids.has(item.id)) {
            ids.delete(item.id)
        } else {
            ids.add(item.id)
        }
        selectedIds.value = new Set(ids)
    }

    function addToSelection(idx: number) {
        if (idx < 0 || idx >= displayMax.value) return
        const item = displayList.value[idx]
        if (!item) return
        const ids = selectedIds.value
        ids.add(item.id)
        selectedIds.value = new Set(ids)
    }

    function removeFromSelection(idx: number) {
        if (idx < 0 || idx >= displayMax.value) return
        const item = displayList.value[idx]
        if (!item) return
        const ids = selectedIds.value
        ids.delete(item.id)
        selectedIds.value = new Set(ids)
    }

    function clearSelection() {
        selectedIds.value = new Set()
    }

    function isSelected(id: string): boolean {
        return selectedIds.value.has(id)
    }

    const selectedItems = computed(() => {
        return displayList.value.filter(item => selectedIds.value.has(item.id))
    })

    /**
     * 重置选区 (query 变化时调用).
     */
    function resetSelection() {
        selectedIndex.value = 0
        selectedGlobalId.value = null
    }

    return {
        selectedIndex,
        selectedGlobalId,
        selectedIds,
        lastSelectedIndex,
        selectedItems,
        selectNext,
        selectPrev,
        selectByIndex,
        selectWithModifiers,
        toggleSelect,
        addToSelection,
        removeFromSelection,
        clearSelection,
        isSelected,
        resetSelection,
    }
}
