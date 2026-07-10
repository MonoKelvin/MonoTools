import { hotkeyManager } from '@/services/hotkeyManager'

interface SearchCommandOptions {
  onEnter?: () => Promise<void>
  onUp?: () => void
  onDown?: () => void
  onEscape?: () => void
}

export const searchCommands = {
  register(options: SearchCommandOptions): void {
    hotkeyManager.register({
      id: 'search.toggle',
      key: 'Alt + Space',
      description: '打开/关闭搜索窗口',
      action: () => {},
      category: '搜索',
    })

    hotkeyManager.register({
      id: 'search.enter',
      key: 'Enter',
      description: '打开选中的项目',
      action: options.onEnter || (() => {}),
      category: '搜索',
    })

    hotkeyManager.register({
      id: 'search.arrowDown',
      key: 'Arrow Down',
      description: '向下选择',
      action: options.onDown || (() => {}),
      category: '搜索',
    })

    hotkeyManager.register({
      id: 'search.arrowUp',
      key: 'Arrow Up',
      description: '向上选择',
      action: options.onUp || (() => {}),
      category: '搜索',
    })

    hotkeyManager.register({
      id: 'search.escape',
      key: 'Escape',
      description: '关闭搜索窗口',
      action: options.onEscape || (() => {}),
      category: '搜索',
    })

    hotkeyManager.register({
      id: 'search.tab',
      key: 'Tab',
      description: '切换分类',
      action: () => {},
      category: '搜索',
    })

    hotkeyManager.register({
      id: 'search.ctrlEnter',
      key: 'Ctrl + Enter',
      description: '打开文件所在路径',
      action: () => {},
      category: '搜索',
    })

    hotkeyManager.register({
      id: 'search.ctrlC',
      key: 'Ctrl + C',
      description: '复制完整路径',
      action: () => {},
      category: '搜索',
    })
  },

  unregister(): void {
    const keys = [
      'search.toggle',
      'search.enter',
      'search.arrowDown',
      'search.arrowUp',
      'search.escape',
      'search.tab',
      'search.ctrlEnter',
      'search.ctrlC',
    ]
    keys.forEach(id => hotkeyManager.unregister(id))
  },
}