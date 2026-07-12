import type { SearchResult } from '@/types/search'

/**
 * 按 score 降序比较. 入参 `a/b` 只需要含 `score?: number` 形状,
 * 不强制必须是完整 SearchResult —— 让单测能用极简 `{ id, score }` 桩.
 */
export function sortByScoreDesc(
  a: Pick<SearchResult, 'score'>,
  b: Pick<SearchResult, 'score'>,
): number {
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
