export interface SearchResult {
  id: string
  title: string
  subtitle?: string
  icon?: string
  source: string
  pluginId: string
  score: number
  actions: ResultAction[]
}

export interface ResultAction {
  id: string
  title: string
  shortcut?: string
  handler?: string
}

export interface SearchQuery {
  raw: string
  prefix?: string
  tokens: string[]
  is_empty: boolean
}

export interface SearchProvider {
  id: string
  name: string
  triggers?: string[]
  priority: number
}
