import { describe, expect, it } from 'vitest'
import { sortByScoreDesc, dedupeById } from '@/utils/sort'

describe('sortByScoreDesc', () => {
  it('orders higher score first', () => {
    const data = [
      { id: 'a', score: 0.2 },
      { id: 'b', score: 0.9 },
      { id: 'c', score: 0.5 },
    ]
    const sorted = [...data].sort(sortByScoreDesc)
    expect(sorted.map((x) => x.id)).toEqual(['b', 'c', 'a'])
  })

  it('treats missing score as 0', () => {
    expect(sortByScoreDesc({ id: '1', score: 0 } as any, { id: '2', score: 0 } as any)).toBe(0)
  })

  it('places higher score first', () => {
    const data = [
      { id: 'a', score: 0.7 },
      { id: 'b', score: 0.2 },
    ]
    const sorted = [...data].sort(sortByScoreDesc)
    expect(sorted[0].id).toBe('a')
    expect(sorted[1].id).toBe('b')
  })
})

describe('dedupeById', () => {
  it('removes items with duplicate id, preserving first occurrence', () => {
    const items = [
      { id: 'a', n: 1 },
      { id: 'b', n: 2 },
      { id: 'a', n: 3 },
    ]
    const out = dedupeById(items)
    expect(out).toHaveLength(2)
    expect(out[0]).toEqual({ id: 'a', n: 1 })
    expect(out[1]).toEqual({ id: 'b', n: 2 })
  })

  it('returns input untouched when no duplicates', () => {
    const items = [{ id: 'a' }, { id: 'b' }]
    const out = dedupeById(items)
    expect(out).toHaveLength(2)
  })
})
