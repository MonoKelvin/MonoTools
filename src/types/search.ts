export type SearchCategory = 'apps' | 'files' | 'commands'

export type ResultType =
  | 'system-app'
  | 'user-app'
  | 'uwp-app'
  | 'directory'
  | 'document'
  | 'image'
  | 'video'
  | 'audio'
  | 'executable'
  | 'archive'
  | 'other-file'
  | 'command'

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
  resultType: ResultType
  action: SearchAction
  score: number
}
