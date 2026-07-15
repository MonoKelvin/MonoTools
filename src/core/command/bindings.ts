/**
 * 键位归一化 + 匹配
 *
 * 注意：与历史 `bindings.ts` 区别：
 *  - 仅保留**纯函数**：可在 tests 里直接构造 ParsedKey 验证。
 *  - 不再依赖 `commandRegistry`：调用方传入 `CommandSpec[]`（一般是 from Pinia store）。
 *  - 不再有 "category 过滤"：调用方决定如何筛选。
 */

const ORDER: ReadonlyArray<'ctrl' | 'alt' | 'shift' | 'meta'> = ['ctrl', 'alt', 'shift', 'meta'] as const

export interface ParsedKey {
  key: string
  ctrl: boolean
  alt: boolean
  shift: boolean
  meta: boolean
}

export function parseKeyEvent(event: KeyboardEvent): ParsedKey {
  return {
    key: event.key,
    ctrl: event.ctrlKey,
    alt: event.altKey,
    shift: event.shiftKey,
    meta: event.metaKey,
  }
}

function toWord(k: 'ctrl' | 'alt' | 'shift' | 'meta'): string {
  switch (k) {
    case 'ctrl':
      return 'Ctrl'
    case 'alt':
      return 'Alt'
    case 'shift':
      return 'Shift'
    case 'meta':
      return 'Meta'
  }
}

function normalizeKeyName(k: string): string {
  const trimmed = k.trim()
  if (trimmed === '' || trimmed.toLowerCase() === 'space') return 'Space'
  if (trimmed.length === 1) return trimmed.toUpperCase()
  return trimmed
}

export function normalizeShortcut(s: string): string {
  const parts = s
    .split('+')
    .map((p) => p.trim())
    .filter(Boolean)
  const mods = new Set<string>()
  let main = ''
  for (const p of parts) {
    const lower = p.toLowerCase()
    if (lower === 'ctrl' || lower === 'control') mods.add('Ctrl')
    else if (lower === 'alt' || lower === 'option') mods.add('Alt')
    else if (lower === 'shift') mods.add('Shift')
    else if (lower === 'meta' || lower === 'cmd' || lower === 'command') mods.add('Meta')
    else main = normalizeKeyName(p)
  }
  const ordered: string[] = []
  for (const k of ORDER) {
    const word = toWord(k)
    if (mods.has(word)) ordered.push(word)
  }
  if (main) ordered.push(main)
  return ordered.join(' + ')
}

export function serializeKey(p: ParsedKey): string {
  const parts: string[] = []
  for (const k of ORDER) if (p[k]) parts.push(toWord(k))
  parts.push(normalizeKeyName(p.key))
  return parts.join(' + ')
}

/**
 * 给定 keyboard event 与已注册 spec 列表，匹配符合的第一条并返回其 id。
 *
 * 选择策略：先按数组顺序遍历 → 第一条命中即返回。**不**做优先级排序（保持与命令注册顺序一致）。
 */
export function dispatchKeyEvent(
  event: KeyboardEvent,
  specs: ReadonlyArray<{ id: string; shortcut?: string | ReadonlyArray<string> }>,
): string | null {
  const evtKey = serializeKey(parseKeyEvent(event))
  for (const s of specs) {
    if (!s.shortcut) continue
    const list = Array.isArray(s.shortcut) ? s.shortcut : [s.shortcut as string]
    for (const combo of list) {
      if (normalizeShortcut(combo) === evtKey) return s.id
    }
  }
  return null
}
