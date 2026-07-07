import type { StartupItem } from './startup'

export interface CustomCommand {
  id: string
  name: string
  description: string | null
  keyword: string
  command: string
  args: string[]
  workingDir: string | null
  icon: string | null
  category: string
  enabled: boolean
  runAsAdmin: boolean
  createdAt: number
  lastUsedAt: number | null
}

export interface SearchHistoryEntry {
  query: string
  resultCount: number
  selectedResultId: string | null
  timestamp: number
}

export interface AppStat {
  appPath: string
  launchCount: number
  lastLaunched: number
  name: string
}

export type { StartupItem }
