export interface HotkeyBinding {
  id: string
  key: string
  description: string
  category: string
  /** 可选：前端可点击触发的回调（仅为 UI 上 "show-hotkeys" 等命令使用） */
  action?: () => void
}

class HotkeyManager {
  private bindings: Map<string, HotkeyBinding> = new Map()
  private enabled: boolean = true

  register(binding: HotkeyBinding): void {
    if (this.bindings.has(binding.id)) {
      return
    }
    this.bindings.set(binding.id, binding)
  }

  unregister(id: string): void {
    this.bindings.delete(id)
  }

  /**
   * 对一组命令 spec（含 shortcut）进行注册。
   * 同一 id 重新注册会覆盖之前的描述（同一快捷键可在不同上下文复用）。
   */
  registerFromSpecs(
    specs: ReadonlyArray<{
      id: string
      title?: string
      description?: string
      category?: string
      shortcut?: string | ReadonlyArray<string>
    }>,
  ): void {
    for (const spec of specs) {
      if (!spec.shortcut) continue
      const list = Array.isArray(spec.shortcut) ? spec.shortcut : [spec.shortcut as string]
      for (const combo of list) {
        const binding: HotkeyBinding = {
          id: spec.id,
          key: combo,
          description: spec.description || spec.title || spec.id,
          category: spec.category || 'system',
        }
        this.bindings.set(spec.id, binding)
      }
    }
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
    if (binding && this.enabled && binding.action) {
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
