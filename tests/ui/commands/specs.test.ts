import { describe, expect, it } from 'vitest'
import { buildBuiltinCommandSpecs, builtinCommandSpecs } from '@/commands/specs'

describe('buildBuiltinCommandSpecs', () => {
  it('id 与后端命令对齐', () => {
    const ids = new Set(builtinCommandSpecs.map((s) => s.id))
    // 后端 9 个主名 + UIonly 的 search.cmd.* / app.cmd.* / theme.cmd.*
    const expected = [
      'search',
      'launch',
      'open',
      'command',
      'config',
      'help',
      'version',
      'index',
      'stats',
      'search.cmd.next-item',
      'search.cmd.prev-item',
      'search.cmd.execute-selected',
      'search.cmd.close-window',
      'search.cmd.toggle-window',
      'search.cmd.focus-input',
      'search.cmd.clear-input',
      'search.cmd.copy-selected-path',
      'search.cmd.reveal-selected',
      'app.cmd.navigate.settings',
      'app.cmd.navigate.commands',
      'app.cmd.quit',
      'app.cmd.show-hotkeys',
      'theme.cmd.toggle',
    ]
    for (const e of expected) {
      expect(ids.has(e), `缺少 ${e}`).toBe(true)
    }
  })

  it('不包含 `.run` 等实现细节 — 只是 metadata', () => {
    const specs = buildBuiltinCommandSpecs()
    for (const s of specs) {
      // @ts-expect-error introspection
      expect(s.run).toBeUndefined()
      // @ts-expect-error introspection
      expect(s.when).toBeUndefined()
      expect(s.id.length).toBeGreaterThan(0)
      expect(s.title.length).toBeGreaterThan(0)
    }
  })

  it('每个后台命令 id 唯一', () => {
    const ids = builtinCommandSpecs.map((s) => s.id)
    expect(new Set(ids).size).toBe(ids.length)
  })

  it('BuiltinCommandSpecs 工厂 = builtinCommandSpecs', () => {
    expect(buildBuiltinCommandSpecs()).toEqual(builtinCommandSpecs)
  })
})
