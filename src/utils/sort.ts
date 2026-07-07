import type { SearchResult } from '@/types/search'

export function sortByScoreDesc(a: SearchResult, b: SearchResult): number {
  return (b.score ?? 0) - (a.score ?? 0)
}

export function dedupeById<T extends { id: string }>(items: T[]): T[] {
  const seen = new Set<string>()
  const out: T[] = []
  for (const it of items) {
    if (!seen.has(it.id)) {
      seen.add(it.id)
      out.push(it)
    }
  }
  return out
}
