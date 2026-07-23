export type SearchCategory = 'apps' | 'files' | 'commands'

export type ResultType =
    | 'system-app'
    | 'user-app'
    | 'uwp-app'
    | 'directory'
    | 'document'
    | 'image'
    | 'svg'
    | 'video'
    | 'audio'
    | 'executable'
    | 'library'
    | 'static-lib'
    | 'dynamic-lib'
    | 'archive'
    | 'shortcut'
    | 'html'
    | 'font'
    | 'config'
    | 'code'
    | 'other-file'
    | 'command'

export interface SearchOptions {
    categories: SearchCategory[]
    limit: number
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
    meta?: string | null
    icon: string | null
    category: SearchCategory
    resultType: ResultType
    action: SearchAction
    score: number
    size?: number | null
    modifiedAt?: number | null
    launchCount?: number | null
}
