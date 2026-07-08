export type SearchCategory = 'apps' | 'files' | 'commands'

export interface SearchOptions {
  categories: SearchCategory[]
  maxResults: number
  includeHidden: boolean
}

export type SearchAction =
  | { type: 'launch'; data: string }
  | { type: 'open'; data: string }
  | { type: 'run'; data: { command: string; args: string[] } }
  | { type: 'navigate'; data: string }

export interface SearchResult {
  id: string
  title: string
  subtitle: string
  icon: string | null
  category: SearchCategory
  action: SearchAction
  score: number
}
