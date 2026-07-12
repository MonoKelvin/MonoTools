import { describe, expect, it, beforeEach, vi } from 'vitest'
import { hotkeyManager } from '@/services/hotkeyManager'
import type { HotkeyBinding } from '@/services/hotkeyManager'

beforeEach(() => {
  hotkeyManager.clear()
  hotkeyManager.setEnabled(true)
})

function binding(id: string): HotkeyBinding {
  return {
    id,
    key: 'A',
    description: id,
    category: 'test',
    action: () => undefined,
  }
}

describe('hotkeyManager', () => {
  it('register adds and getAll lists bindings', () => {
    hotkeyManager.register(binding('h1'))
    hotkeyManager.register(binding('h2'))
    expect(hotkeyManager.getAll()).toHaveLength(2)
    expect(hotkeyManager.getById('h1')?.id).toBe('h1')
  })

  it('register is idempotent (silently ignores)', () => {
    hotkeyManager.register(binding('dup'))
    hotkeyManager.register(binding('dup'))
    expect(hotkeyManager.getAll()).toHaveLength(1)
  })

  it('unregister removes a binding', () => {
    hotkeyManager.register(binding('h1'))
    hotkeyManager.unregister('h1')
    expect(hotkeyManager.getById('h1')).toBeUndefined()
  })

  it('categories lists distinct values', () => {
    hotkeyManager.register({ ...binding('h1'), category: 'a' })
    hotkeyManager.register({ ...binding('h2'), category: 'b' })
    hotkeyManager.register({ ...binding('h3'), category: 'a' })
    expect(hotkeyManager.getCategories().sort()).toEqual(['a', 'b'])
  })

  it('getByCategory filters by category', () => {
    hotkeyManager.register({ ...binding('h1'), category: 'a' })
    hotkeyManager.register({ ...binding('h2'), category: 'b' })
    expect(hotkeyManager.getByCategory('a')).toHaveLength(1)
    expect(hotkeyManager.getByCategory('b')).toHaveLength(1)
  })

  it('execute invokes action when enabled', () => {
    const action = vi.fn()
    hotkeyManager.register({ ...binding('a'), action })
    const result = hotkeyManager.execute('a')
    expect(result).toBe(true)
    expect(action).toHaveBeenCalled()
  })

  it('execute returns false when disabled', () => {
    const action = vi.fn()
    hotkeyManager.register({ ...binding('a'), action })
    hotkeyManager.setEnabled(false)
    const result = hotkeyManager.execute('a')
    expect(result).toBe(false)
    expect(action).not.toHaveBeenCalled()
  })

  it('execute returns false when id missing', () => {
    expect(hotkeyManager.execute('missing')).toBe(false)
  })

  it('clear empties bindings', () => {
    hotkeyManager.register(binding('h1'))
    hotkeyManager.clear()
    expect(hotkeyManager.getAll()).toHaveLength(0)
  })
})
