export type ThemeMode = 'light' | 'dark' | 'auto'

export interface Settings {
  hotkey: string
  theme: ThemeMode
  accentColor: string
  fileSearchEnabled: boolean
  fileSearchRoots: string[]
  enabledCategories: string[]
  pinToTop: boolean
}