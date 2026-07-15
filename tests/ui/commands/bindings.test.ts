import { describe, expect, it } from 'vitest'
import {
  normalizeShortcut,
  serializeKey,
  parseKeyEvent,
  dispatchKeyEvent,
} from '@/core/command/bindings'

function evt(opts: {
  key: string
  ctrl?: boolean
  alt?: boolean
  shift?: boolean
  meta?: boolean
}): KeyboardEvent {
  return {
    key: opts.key,
    ctrlKey: !!opts.ctrl,
    altKey: !!opts.alt,
    shiftKey: !!opts.shift,
    metaKey: !!opts.meta,
  } as unknown as KeyboardEvent
}

describe('keyboard shortcut normalization', () => {
  it('matches Enter', () => {
    expect(dispatchKeyEvent(evt({ key: 'Enter' }), [{ id: 'enter', shortcut: 'Enter' }])).toBe('enter')
  })

  it('matches Ctrl+Enter with normalized modifier', () => {
    expect(
      dispatchKeyEvent(evt({ key: 'Enter', ctrl: true }), [{ id: 'ce', shortcut: 'Ctrl + Enter' }]),
    ).toBe('ce')
  })

  it('matches Alt+Space', () => {
    expect(
      dispatchKeyEvent(evt({ key: ' ', alt: true }), [{ id: 'as', shortcut: 'Alt + Space' }]),
    ).toBe('as')
  })

  it('rejects missing modifier', () => {
    expect(
      dispatchKeyEvent(evt({ key: 'c' }), [{ id: 'cc', shortcut: 'Ctrl + C' }]),
    ).toBeNull()
  })

  it('rejects different main key', () => {
    expect(
      dispatchKeyEvent(evt({ key: 'Escape' }), [{ id: 'en', shortcut: 'Enter' }]),
    ).toBeNull()
  })

  it('uppercases single-letter keys', () => {
    expect(serializeKey(parseKeyEvent(evt({ key: 'a' })))).toBe('A')
  })

  it('normalizes Space', () => {
    expect(serializeKey(parseKeyEvent(evt({ key: ' ' })))).toBe('Space')
  })

  it('serializes Ctrl before Alt before Shift before Meta', () => {
    expect(
      serializeKey(parseKeyEvent(evt({ key: 't', shift: true, alt: true, ctrl: true }))),
    ).toBe('Ctrl + Alt + Shift + T')
  })

  it('normalizes Cmd -> Meta', () => {
    expect(normalizeShortcut('Cmd+Enter')).toBe('Meta + Enter')
  })

  it('normalizes Control -> Ctrl', () => {
    expect(normalizeShortcut('Control+Shift+K')).toBe('Ctrl + Shift + K')
  })

  it('normalizes alt+space case-insensitively', () => {
    expect(normalizeShortcut('alt + space')).toBe('Alt + Space')
  })
})

describe('dispatchKeyEvent — first match wins', () => {
  const specs = [
    { id: 'a.first', shortcut: 'Ctrl + K' },
    { id: 'b.also_ctrl_k', shortcut: 'Ctrl + K' },
    { id: 'c.something_else', shortcut: 'Enter' },
  ]

  it('returns null on no match', () => {
    const result = dispatchKeyEvent(evt({ key: 'q' }), specs)
    expect(result).toBeNull()
  })

  it('returns null when no shortcut in spec', () => {
    const result = dispatchKeyEvent(evt({ key: 'a' }), [{ id: 'idle' }])
    expect(result).toBeNull()
  })

  it('returns the first spec whose shortcut matches', () => {
    expect(dispatchKeyEvent(evt({ key: 'k', ctrl: true }), specs)).toBe('a.first')
    expect(dispatchKeyEvent(evt({ key: 'Enter' }), specs)).toBe('c.something_else')
  })

  it('handles spec.shortcut as a list', () => {
    const multi = [{ id: 'multi', shortcut: ['Ctrl + K', 'Ctrl + Shift + K'] }]
    expect(dispatchKeyEvent(evt({ key: 'k', ctrl: true }), multi)).toBe('multi')
    expect(dispatchKeyEvent(evt({ key: 'k', ctrl: true, shift: true }), multi)).toBe('multi')
    expect(dispatchKeyEvent(evt({ key: 'k' }), multi)).toBeNull()
  })
})
