import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { StartupItem, NewStartupItem } from '@/types/startup'
import { startupApi } from '@/services/startupApi'

export const useStartupStore = defineStore('startup', () => {
  const items = ref<StartupItem[]>([])
  const loading = ref(false)

  async function refresh() {
    loading.value = true
    try {
      items.value = await startupApi.list()
    } finally {
      loading.value = false
    }
  }

  async function toggle(id: string, enabled: boolean) {
    await startupApi.toggle(id, enabled)
    await refresh()
  }

  async function add(item: NewStartupItem) {
    await startupApi.add(item)
    await refresh()
  }

  async function remove(id: string) {
    await startupApi.remove(id)
    await refresh()
  }

  async function update(item: StartupItem) {
    await startupApi.update(item)
    await refresh()
  }

  return { items, loading, refresh, toggle, add, remove, update }
})
