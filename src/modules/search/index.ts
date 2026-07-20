import { commandSpecRegistry } from '@/core/command'
import { searchCommandSpecs } from './commandSpecs'

commandSpecRegistry.register(searchCommandSpecs)

export { useSearchStore, GROUP_ID, type ActiveCategory, type IndexStatus, type GroupId, type DisplayGroup } from './store'
export * from './types'
export { useSearchStatusBar } from './composables/useSearchStatusBar'
export { useStatusMessages } from './composables/useStatusMessages'
export { searchCommandSpecs } from './commandSpecs'

export { default as SearchPage } from './pages/SearchPage.vue'
export { default as SearchInput } from './components/SearchInput.vue'
export { default as ResultItem } from './components/ResultItem.vue'
export { default as AppResultItem } from './components/AppResultItem.vue'
export { default as ActionBar } from './components/ActionBar.vue'
export { default as GroupSection } from './components/GroupSection.vue'
export { default as ContextMenu } from './components/ContextMenu.vue'
