export interface HotkeyBinding {
  id: string
  key: string
  description: string
  action: () => void
  category: string
}

class HotkeyManager {
  private bindings: Map<string, HotkeyBinding> = new Map()
  private enabled: boolean = true

  register(binding: HotkeyBinding): void {
    if (this.bindings.has(binding.id)) {
      console.warn(`快捷键已注册: ${binding.id}`)
      return
    }
    this.bindings.set(binding.id, binding)
  }

  unregister(id: string): void {
    this.bindings.delete(id)
  }

  getById(id: string): HotkeyBinding | undefined {
    return this.bindings.get(id)
  }

  getAll(): HotkeyBinding[] {
    return Array.from(this.bindings.values())
  }

  getByCategory(category: string): HotkeyBinding[] {
    return this.getAll().filter(b => b.category === category)
  }

  getCategories(): string[] {
    const categories = new Set<string>()
    this.bindings.forEach(b => categories.add(b.category))
    return Array.from(categories)
  }

  execute(id: string): boolean {
    const binding = this.bindings.get(id)
    if (binding && this.enabled) {
      binding.action()
      return true
    }
    return false
  }

  setEnabled(enabled: boolean): void {
    this.enabled = enabled
  }

  isEnabled(): boolean {
    return this.enabled
  }

  clear(): void {
    this.bindings.clear()
  }
}

export const hotkeyManager = new HotkeyManager()