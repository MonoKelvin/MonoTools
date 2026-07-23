/**
 * 搜索场景状态栏编排器.
 *
 * 职责: 把搜索相关的业务状态 (索引构建 / 盘符进度 / 选中项 / 结果统计 /
 * 临时消息) 翻译成**通用**的 {@link StatusBarMessage}, 交给 ActionBar
 * 这个纯展示组件渲染. ActionBar 本身不感知任何搜索业务.
 *
 * 优先级 (高 → 低):
 *  1. 临时消息管理器 (useStatusMessages) —— 例如右键菜单 "已固定" 反馈
 *  2. 索引状态 (building / completed / error), 含多盘符进度
 *  3. 选中项信息 / 结果统计
 *
 * 设计要点:
 * - 索引 building 常驻显示; completed 5s / error 8s 后自动隐藏 (定时器在此托管).
 * - loading 态用粗粒度 id (按卷序号), 避免索引计数刷新反复触发 ActionBar 过渡.
 * - 选中项的 resultType 缺失时按 category 兜底, 杜绝 "undefined".
 */

import { computed, onUnmounted, ref, watch } from 'vue'
import { ACTION_BAR_TIMEOUTS } from '@/core/config'
import { resultTypeMeta } from '../utils/resultTypeMeta'
import { useStatusMessages, type StatusMessage } from '@/modules/search/composables/useStatusMessages'
import { useSearchStore } from '@/modules/search'
import type {
  StatusBarMessage,
  StatusBarType,
  StatusSegment,
  StatusSegmentKind,
} from '@/core/types/statusBar'

/** category 兜底标签: resultType 缺失时按 category 给语义化标签. */
const CATEGORY_FALLBACK: Record<string, string> = {
  apps: '应用程序',
  files: '文件',
  commands: '命令',
}

/** 多盘符索引进度结构. */
interface VolumeProgress {
  idx: number
  total: number
  cur: string
}

/** 临时消息 type → 通用 StatusBarType. */
function messageToBarType(t: StatusMessage['type']): StatusBarType {
  switch (t) {
    case 'success':
      return 'success'
    case 'error':
      return 'error'
    case 'loading':
    case 'building':
      return 'loading'
    case 'info':
    default:
      return 'info'
  }
}

/** 索引状态字符串 → 通用 StatusBarType. null 表示不展示. */
function indexStatusToType(s: string): StatusBarType | null {
  switch (s) {
    case 'building':
      return 'loading'
    case 'completed':
      return 'success'
    case 'error':
      return 'error'
    case 'idle':
      return 'info'
    default:
      return null
  }
}

/**
 * 把含数字的文本拆成片段, 数字部分高亮为 number 态.
 * 用于后端拼好的字符串 (如 "已索引 123,456 个文件").
 */
function highlightNumbers(text: string, baseKind: StatusSegmentKind = 'label'): StatusSegment[] {
  if (!text) return []
  const out: StatusSegment[] = []
  const re = /(\d[\d,.]*)/g
  let last = 0
  let m: RegExpExecArray | null
  while ((m = re.exec(text)) !== null) {
    if (m.index > last) out.push({ text: text.slice(last, m.index), kind: baseKind })
    out.push({ text: m[0], kind: 'number' })
    last = re.lastIndex
  }
  if (last < text.length) out.push({ text: text.slice(last), kind: baseKind })
  return out.length ? out : [{ text, kind: baseKind }]
}

/**
 * @param search 搜索 store 实例. 传入而非内部 use, 便于测试与显式依赖.
 */
export function useSearchStatusBar(search: ReturnType<typeof useSearchStore>) {
  const { currentMessage: statusMessage, addMessage, removeMessage, clearMessages } =
    useStatusMessages()

  // === 索引状态显隐托管 ============================================
  // building / idle 常驻; completed 5s / error 8s 后隐藏.
  const showIndexStatus = ref(false)
  const autoHideTimer = ref<number | null>(null)

  function clearAutoHide() {
    if (autoHideTimer.value) {
      clearTimeout(autoHideTimer.value)
      autoHideTimer.value = null
    }
  }

  watch(
    () => search.indexStatus,
    (newStatus) => {
      if (newStatus === 'building' || newStatus === 'idle') {
        showIndexStatus.value = true
        clearAutoHide()
      } else if (newStatus === 'completed') {
        showIndexStatus.value = true
        clearAutoHide()
        autoHideTimer.value = window.setTimeout(() => {
          showIndexStatus.value = false
        }, ACTION_BAR_TIMEOUTS.completedMs)
      } else if (newStatus === 'error') {
        showIndexStatus.value = true
        clearAutoHide()
        autoHideTimer.value = window.setTimeout(() => {
          showIndexStatus.value = false
        }, ACTION_BAR_TIMEOUTS.errorMs)
      }
    },
  )

  onUnmounted(clearAutoHide)

  // === 盘符进度 ====================================================
  const volumeProgress = computed<VolumeProgress | null>(() => {
    const total = search.indexVolumesTotal ?? 0
    const idx = search.indexVolumeIndex ?? 0
    const cur = search.indexCurrentVolume ?? ''
    if (total > 0) return { idx, total, cur }
    return null
  })

  // === 选中项 / 结果统计 ===========================================
  let selectionSeq = 0
  const selectionMessage = computed<StatusBarMessage>(() => {
    // 强制依赖 selectedIndexes 变化 → 多选数量变化时立即重算
    void search.selectedIndexes
    const list = search.displayList
    if (list.length === 0) {
      return { id: `empty-${++selectionSeq}`, type: 'info', segments: [{ text: '未找到结果', kind: 'label' }] }
    }

    const selCount = search.selectedIndexes?.size ?? 0
    // 多选模式: 选中数量 > 1 时显示"已选择 N 项"
    if (selCount > 1) {
      return {
        id: `multi-${selCount}-${++selectionSeq}`,
        type: 'info',
        segments: [
          { text: '已选择 ', kind: 'label' },
          { text: String(selCount), kind: 'number' },
          { text: ' 项', kind: 'label' },
        ],
      }
    }

    const selected = list[search.selectedIndex]
    if (selected) {
      // resultType 缺失 → category 兜底 → 空串, 杜绝 "undefined".
      const typeLabel =
        resultTypeMeta(selected.resultType)?.labelFull ??
        CATEGORY_FALLBACK[selected.category] ??
        ''
      const segs: StatusSegment[] = [
        { text: '已选择 ', kind: 'label' },
        { text: selected.title, kind: 'primary' },
      ]
      if (typeLabel) {
        segs.push({ text: ' · ', kind: 'muted' })
        segs.push({ text: typeLabel, kind: 'accent' })
      }
      // meta = 文件大小 / 安装时间等次级元信息, 高亮为数字态.
      if (selected.meta) {
        segs.push({ text: ' · ', kind: 'muted' })
        segs.push({ text: selected.meta, kind: 'number' })
      }
      return { id: `sel-${selected.id}-${++selectionSeq}`, type: 'info', segments: segs }
    }

    return {
      id: `count-${list.length}-${++selectionSeq}`,
      type: 'info',
      segments: [
        { text: '共 ', kind: 'label' },
        { text: String(list.length), kind: 'number' },
        { text: ' 项结果', kind: 'label' },
      ],
    }
  })

  // === 编排: 三级优先级 ============================================
  const message = computed<StatusBarMessage | null>(() => {
    // 优先级1: 临时消息管理器 (右键反馈 / 主动推送等)
    if (statusMessage.value) {
      const msg = statusMessage.value
      return {
        id: `msg-${msg.id}`,
        type: messageToBarType(msg.type),
        segments: highlightNumbers(msg.text, 'primary'),
      }
    }

    // 优先级2: 索引状态
    if (showIndexStatus.value) {
      const status = search.indexStatus
      // building + 盘符进度: "索引中 1/3 · E:"
      if (status === 'building' && volumeProgress.value) {
        const vp = volumeProgress.value
        return {
          // 粗粒度 id: 同一卷内计数刷新不触发 ActionBar 过渡, 切卷才触发.
          id: `loading-${vp.idx}-${vp.cur}`,
          type: 'loading',
          segments: [
            { text: '索引中 ', kind: 'label' },
            { text: `${vp.idx}/${vp.total}`, kind: 'number' },
            { text: ' · ', kind: 'muted' },
            { text: vp.cur, kind: 'accent' },
          ],
        }
      }
      const t = indexStatusToType(status)
      if (t) {
        return {
          id: `idx-${status}-${search.indexMessage}`,
          type: t,
          segments: highlightNumbers(search.indexMessage, 'label'),
        }
      }
    }

    // 优先级3: 选中项 / 结果统计
    return selectionMessage.value
  })

  return {
    /** 供 ActionBar 渲染的通用消息. */
    message,
    /** 透传消息管理器接口, 便于业务侧统一通过编排器推送临时消息. */
    addMessage,
    removeMessage,
    clearMessages,
  }
}
