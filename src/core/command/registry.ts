import type { CommandSpec } from './types'
import { builtinCommandSpecs } from './specs'

class CommandSpecRegistry {
  private specs: Map<string, CommandSpec> = new Map()
  private registered = false

  ensureDefaultRegistered(): void {
    if (this.registered) return
    this.registered = true
    for (const spec of builtinCommandSpecs) {
      this.specs.set(spec.id, spec)
    }
  }

  register(specs: CommandSpec[]): void {
    this.ensureDefaultRegistered()
    for (const spec of specs) {
      this.specs.set(spec.id, spec)
    }
  }

  unregister(ids: string[]): void {
    for (const id of ids) {
      this.specs.delete(id)
    }
  }

  getAll(): CommandSpec[] {
    this.ensureDefaultRegistered()
    return Array.from(this.specs.values())
  }

  get(id: string): CommandSpec | undefined {
    this.ensureDefaultRegistered()
    return this.specs.get(id)
  }

  clear(): void {
    this.specs.clear()
    this.registered = false
  }
}

export const commandSpecRegistry = new CommandSpecRegistry()
