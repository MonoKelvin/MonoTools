export type StartupSource =
  | 'registryRun'
  | 'registryRunOnce'
  | 'startupFolder'
  | 'scheduledTask'
  | 'custom'

export interface StartupItem {
  id: string
  name: string
  command: string
  args: string[]
  workingDir: string | null
  enabled: boolean
  delaySeconds: number
  runAsAdmin: boolean
  source: StartupSource
  createdAt: number
}

export interface NewStartupItem {
  name: string
  command: string
  args: string[]
  workingDir?: string | null
  delaySeconds: number
  runAsAdmin: boolean
}
