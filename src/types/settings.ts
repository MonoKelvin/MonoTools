export type ThemeMode = 'light' | 'dark' | 'auto'

export interface Settings {
  hotkey: string
  theme: ThemeMode
  accentColor: string
  fileSearchEnabled: boolean
  fileSearchRoots: string[]
  defaultDelaySeconds: number
  autoIndexOnStartup: boolean
  enabledCategories: string[]
  customAppPaths: string[]
}

export interface Appearance {
  mode: ThemeMode
  accent: string
}
