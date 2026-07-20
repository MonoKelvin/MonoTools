/**
 * 命令管理 Pinia store：唯一状态点。
 *
 * 生命周期：
 *   1. 应用 mount 时 `await loadFromBackend()` —— 通过 IPC `list_command_specs` 拉取后端已注册命令
 *   2. 用户按下 hotkey / 托盘 / 菜单：UI 调用 `execute(id, args)` 委托给 IPC `dispatch_command`
 *   3. 组件按需 `get(id) / listByCategory(category) / shortcutOf('Ctrl + K')` 获取纯 metadata 用于渲染
 *
 * 不再做：
 *   - 不再注册自定义 runnable —— 命令业务在后端
 *   - 不再维护 `enabled / when` 标志 —— 由后端的 Command trait / dispatch 决定是否生效
 *   - 不再有 in-memory list（除非 mock 模式） —— 全部从后端拉
 */
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import * as tauriSvc from '@/services/tauri'
import { hotkeyManager } from '@/services/hotkeyManager'
import { normalizeShortcut } from './bindings'
import { commandSpecRegistry } from './registry'
import type { CommandCategory, CommandExecutionResult, CommandSpec } from './types'

const invokeCmd: <T>(cmd: string, args: Record<string, unknown>) => Promise<T> =
  // 显式取自 `@/services/tauri.call`，在测试里通过 `vi.spyOn(tauriSvc, 'call')` 拦截
  // 这里我们也不能用 `tauriSvc.call` 直接绑定（spy 后续还要重新调用原始实现），
  // 因此通过 `(cmd, args) => tauriSvc.call(cmd, args)` 保留动态派发。
  (cmd, args) =>
    (tauriSvc.call as unknown as <T>(
      cmd: string,
      args?: Record<string, unknown>,
    ) => Promise<T>)(cmd, args)

/**
 * 是否在 Tauri 上下文：测试可临时删除 `window.__TAURI__` 强制走 mock 路径。
 */
function isTauriEnv(): boolean {
  return !!(globalThis as any).window?.__TAURI__
}

export interface CommandsBackendSpec {
  name: string
  description?: string
  aliases?: string[]
  usage?: string
}

function backendToSpec(raw: CommandsBackendSpec): CommandSpec {
  const aliases = Array.isArray(raw?.aliases) ? raw.aliases : []
  // 后端只给 name / description / aliases / usage，前端自己给元数据保留默认 category / icon / keywords
  return {
    id: raw.name,
    title: raw.description ?? raw.name,
    description: raw.description,
    category: 'system',
    shortcut: undefined,
    icon: undefined,
    keywords: aliases,
  }
}

export const useCommandsStore = defineStore('commands', () => {
  const specs = ref<CommandSpec[]>([])
  const isLoaded = ref(false)
  const errorListeners: Array<(err: unknown, id: string) => void> = []

  // 公开 `lastError` 让外部能看到失败原因：
  // null = 还没尝试过加载；string = 最近一次错误信息（"" 表示加载成功）
  const lastError = ref<string | null>(null)

  /** 把当前 specs 的 shortcut 同步到 hotkeyManager（用于 HotkeyModal 显示）。 */
  function syncHotkeyManager() {
    hotkeyManager.clear()
    hotkeyManager.registerFromSpecs(specs.value)
  }

  // specs 变化触发 hotkeyManager 同步，无论来自 loadFromBackend / override / reset
  watch(specs, syncHotkeyManager, { deep: false })

  function snapshot(): CommandSpec[] {
    return specs.value.slice()
  }

  async function loadFromBackend(force = false): Promise<void> {
    if (isLoaded.value && !force) return
    if (!isTauriEnv()) {
      specs.value = commandSpecRegistry.getAll().map((s) => ({ ...s }))
      isLoaded.value = true
      return
    }
    try {
      const raw = await invokeCmd<CommandsBackendSpec[]>('list_command_specs', {})
      const list = Array.isArray(raw) ? raw : []
      const map = new Map<string, CommandSpec>()
      for (const b of list) {
        const id = b?.name
        if (!id) continue
        const spec = backendToSpec({
          name: id,
          description: b?.description,
          aliases: b?.aliases,
          usage: b?.usage,
        })
        map.set(id, spec)
      }
      for (const ui of commandSpecRegistry.getAll()) {
        if (!map.has(ui.id)) map.set(ui.id, ui)
      }
      specs.value = Array.from(map.values())
      isLoaded.value = true
      lastError.value = null
    } catch (err) {
      specs.value = commandSpecRegistry.getAll().map((s) => ({ ...s }))
      isLoaded.value = true
      const msg = err instanceof Error ? err.message : String(err)
      lastError.value = msg || 'unknown'
    }
  }

  function override(specsPatch: CommandSpec[]): void {
    // 给 tests / mock 用：直接覆盖 store state，绕过后端拉取
    specs.value = specsPatch
    isLoaded.value = true
  }

  function reset(): void {
    specs.value = []
    isLoaded.value = false
    lastError.value = null
  }

  function get(id: string): CommandSpec | undefined {
    return specs.value.find((s) => s.id === id || (s.keywords ?? []).includes(id))
  }

  function list(): CommandSpec[] {
    return snapshot()
  }

  function listByCategory(category: CommandCategory): CommandSpec[] {
    return specs.value.filter((s) => s.category === category)
  }

  const categories = computed<CommandCategory[]>(() => {
    const set = new Set<CommandCategory>()
    for (const s of specs.value) set.add(s.category)
    return Array.from(set)
  })

  /** 编辑型 shortcut 归一化只匹配主段（与后端命令里的"主快捷键"对齐） */
  function shortcutMatches(spec: CommandSpec, combo: string): boolean {
    if (!spec.shortcut) return false
    const list = Array.isArray(spec.shortcut) ? spec.shortcut : [spec.shortcut as string]
    return list.some((c) => normalizeShortcut(c) === normalizeShortcut(combo))
  }

  /**
   * 在已注册 spec 中找到与 `combo`（规范化的快捷键字符串）匹配的第一条命令。
   * 用于键盘 keydown 处理；返回 id 而不是 spec，避免调用方重复归一化。
   */
  function matchShortcut(combo: string): CommandSpec | undefined {
    return specs.value.find((s) => shortcutMatches(s, combo))
  }

  async function execute(
    id: string,
    args: string[] = [],
  ): Promise<CommandExecutionResult | undefined> {
    if (!isTauriEnv()) {
      return { success: true, message: `mock: ${id}`, data: undefined } as CommandExecutionResult
    }
    let out: CommandExecutionResult | null = null
    try {
      const result = await invokeCmd<CommandExecutionResult | null>('dispatch_command', {
        commandId: id,
        args,
      })
      out = result as CommandExecutionResult | null
    } catch (err) {
      for (const l of errorListeners) l(err, id)
      const msg = err instanceof Error ? err.message : String(err)
      lastError.value = msg
    }
    if (!out) {
      return undefined
    }
    if (!out?.success) {
      const err = new Error(out.message ?? `${id} failed`)
      for (const l of errorListeners) l(err, id)
    }
    return out
  }

  function onError(listener: (err: unknown, id: string) => void): () => void {
    errorListeners.push(listener)
    return () => {
      const idx = errorListeners.indexOf(listener)
      if (idx >= 0) errorListeners.splice(idx, 1)
    }
  }

  return {
    specs,
    isLoaded,
    lastError,
    loadFromBackend,
    override,
    reset,
    get,
    list,
    listByCategory,
    categories,
    matchShortcut,
    execute,
    onError,
  }
})
