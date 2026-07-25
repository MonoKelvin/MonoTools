import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { isTauri } from './env'

/**
 * 统一封装 Tauri 后端调用。
 * 在开发期的浏览器（非 Tauri 环境）下，提供 mock 数据，便于纯前端调试。
 */

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    return mockBackend<T>(cmd, args)
  }
  try {
    return await invoke<T>(cmd, args)
  } catch (err) {
    console.error(`invoke(${cmd}) failed:`, err)
    throw err
  }
}

export async function listenEvent<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  if (!isTauri) {
    return () => {
      /* noop */
    }
  }
  return listen<T>(event, (e) => handler(e.payload))
}

const store = new Map<string, unknown>()
function mockGet<T>(key: string): T | undefined {
  return store.get(key) as T | undefined
}
function mockSet<T>(key: string, value: T): void {
  store.set(key, value)
}

async function mockBackend<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await new Promise((r) => setTimeout(r, 12))
  switch (cmd) {
    case 'search':
    case 'search_cmd':
      return mockSearch((args?.query as string) ?? '') as T
    case 'get_setting':
      return mockGet(args?.key as string) as T
    case 'set_setting':
      mockSet(args?.key as string, args?.value)
      return true as T
    case 'list_commands':
      return [] as T
    case 'list_command_specs': {
      const specs = [
        { name: 'search', description: '搜索', aliases: ['s', 'find'], usage: 'search <query>' },
        { name: 'launch', description: '启动', aliases: ['run', 'open-app'], usage: 'launch <name>' },
        { name: 'open', description: '打开', aliases: [], usage: 'open <path>' },
        { name: 'config', description: '配置', aliases: ['cfg', 'setting'], usage: 'config' },
        { name: 'help', description: '帮助', aliases: ['-h', '--help'], usage: 'help' },
        { name: 'version', description: '版本', aliases: ['-v', '--version'], usage: 'version' },
      ] as unknown as T
      return specs
    }
    case 'dispatch_command':
      return { success: true, message: 'mock', data: undefined } as T
    case 'show_search_window':
      return true as T
    case 'hide_search_window':
      return true as T
    case 'register_hotkey':
      return true as T
    case 'frontend_ready':
      return true as T
    case 'get_index_status':
      return { files: 2297401, apps: 186, commands: 6 } as T
    case 'build_file_index':
      return '索引构建已启动' as T
    case 'get_settings_bulk':
      return {} as T
    case 'set_settings_bulk':
      return true as T
    default:
      return ([] as unknown) as T
  }
}

function mockSearch(query: string) {
  const lower = query.toLowerCase()
  const allApps = [
    { name: 'Google Chrome', subtitle: 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe', category: 'apps', resultType: 'user-app' },
    { name: 'Visual Studio Code', subtitle: 'C:\\Users\\MONO\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe', category: 'apps', resultType: 'user-app' },
    { name: 'Spotify', subtitle: 'C:\\Users\\MONO\\AppData\\Roaming\\Spotify\\Spotify.exe', category: 'apps', resultType: 'user-app' },
    { name: 'Microsoft Edge', subtitle: 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe', category: 'apps', resultType: 'system-app' },
    { name: 'Windows Terminal', subtitle: 'C:\\Program Files\\WindowsApps\\Microsoft.WindowsTerminal\\wt.exe', category: 'apps', resultType: 'uwp-app' },
    { name: 'File Explorer', subtitle: 'C:\\Windows\\explorer.exe', category: 'apps', resultType: 'system-app' },
    { name: 'Notepad', subtitle: 'C:\\Windows\\System32\\notepad.exe', category: 'apps', resultType: 'system-app' },
    { name: 'Calculator', subtitle: 'C:\\Windows\\System32\\calc.exe', category: 'apps', resultType: 'system-app' },
    { name: 'Paint', subtitle: 'C:\\Windows\\System32\\mspaint.exe', category: 'apps', resultType: 'system-app' },
    { name: 'Settings', subtitle: 'C:\\Windows\\ImmersiveControlPanel\\SystemSettings.exe', category: 'apps', resultType: 'system-app' },
    { name: 'PowerShell', subtitle: 'C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe', category: 'apps', resultType: 'system-app' },
    { name: 'Task Manager', subtitle: 'C:\\Windows\\System32\\Taskmgr.exe', category: 'apps', resultType: 'system-app' },
    { name: 'Discord', subtitle: 'C:\\Users\\MONO\\AppData\\Local\\Discord\\Update.exe', category: 'apps', resultType: 'user-app' },
    { name: 'Slack', subtitle: 'C:\\Users\\MONO\\AppData\\Local\\slack\\slack.exe', category: 'apps', resultType: 'user-app' },
    { name: 'Adobe Photoshop', subtitle: 'C:\\Program Files\\Adobe\\Photoshop\\Photoshop.exe', category: 'apps', resultType: 'user-app' },
    { name: 'Figma', subtitle: 'C:\\Users\\MONO\\AppData\\Local\\Figma\\Figma.exe', category: 'apps', resultType: 'user-app' },
    { name: 'Obsidian', subtitle: 'C:\\Users\\MONO\\AppData\\Local\\Obsidian\\Obsidian.exe', category: 'apps', resultType: 'user-app' },
    { name: 'Notion', subtitle: 'C:\\Users\\MONO\\AppData\\Local\\Notion\\Notion.exe', category: 'apps', resultType: 'user-app' },
  ]
  const allFiles = [
    { name: 'README.md', subtitle: 'E:\\work\\code\\MonoTools\\README.md', category: 'files', resultType: 'document' },
    { name: 'package.json', subtitle: 'E:\\work\\code\\MonoTools\\package.json', category: 'files', resultType: 'code' },
    { name: 'index.html', subtitle: 'E:\\work\\code\\MonoTools\\index.html', category: 'files', resultType: 'html' },
    { name: 'design.png', subtitle: 'D:\\design\\hero\\design.png', category: 'files', resultType: 'image' },
    { name: 'screenshot.jpg', subtitle: 'D:\\design\\screenshots\\screenshot.jpg', category: 'files', resultType: 'image' },
    { name: 'logo.svg', subtitle: 'E:\\work\\code\\MonoTools\\src\\logo.svg', category: 'files', resultType: 'svg' },
    { name: 'data.csv', subtitle: 'D:\\data\\exports\\data.csv', category: 'files', resultType: 'document' },
    { name: 'report.pdf', subtitle: 'D:\\documents\\reports\\report.pdf', category: 'files', resultType: 'document' },
    { name: 'presentation.pptx', subtitle: 'D:\\documents\\presentation.pptx', category: 'files', resultType: 'document' },
    { name: 'main.rs', subtitle: 'E:\\work\\code\\MonoTools\\src-tauri\\src\\main.rs', category: 'files', resultType: 'code' },
    { name: 'app.vue', subtitle: 'E:\\work\\code\\MonoTools\\src\\App.vue', category: 'files', resultType: 'code' },
    { name: 'demo.mp4', subtitle: 'D:\\videos\\demo.mp4', category: 'files', resultType: 'video' },
    { name: 'song.mp3', subtitle: 'D:\\music\\song.mp3', category: 'files', resultType: 'audio' },
    { name: 'archive.zip', subtitle: 'D:\\downloads\\archive.zip', category: 'files', resultType: 'archive' },
    { name: 'installer.exe', subtitle: 'D:\\downloads\\installer.exe', category: 'files', resultType: 'executable' },
    { name: 'lib.dll', subtitle: 'C:\\Windows\\System32\\lib.dll', category: 'files', resultType: 'library' },
    { name: 'lib.a', subtitle: 'E:\\libs\\liba.a', category: 'files', resultType: 'library' },
    { name: 'image.bmp', subtitle: 'C:\\Users\\MONO\\Pictures\\image.bmp', category: 'files', resultType: 'image' },
    { name: 'notes.txt', subtitle: 'C:\\Users\\MONO\\Documents\\notes.txt', category: 'files', resultType: 'document' },
    { name: 'config.json', subtitle: 'E:\\work\\config.json', category: 'files', resultType: 'code' },
    { name: 'setup.ini', subtitle: 'D:\\app\\config.ini', category: 'files', resultType: 'config' },
    { name: 'settings.yaml', subtitle: 'D:\\app\\settings.yaml', category: 'files', resultType: 'config' },
    { name: 'font.ttf', subtitle: 'C:\\Fonts\\font.ttf', category: 'files', resultType: 'font' },
    { name: 'shortcut.lnk', subtitle: 'C:\\Users\\MONO\\Desktop\\shortcut.lnk', category: 'files', resultType: 'shortcut' },
    { name: 'page.html', subtitle: 'D:\\www\\page.html', category: 'files', resultType: 'html' },
  ]
  const allCommands = [
    { name: 'git status', subtitle: 'Show working tree status', category: 'commands', resultType: 'command' },
    { name: 'git commit -m ""', subtitle: 'Record changes to the repository', category: 'commands', resultType: 'command' },
    { name: 'pnpm install', subtitle: 'Install dependencies', category: 'commands', resultType: 'command' },
    { name: 'pnpm dev', subtitle: 'Run dev server', category: 'commands', resultType: 'command' },
    { name: 'pnpm build', subtitle: 'Build for production', category: 'commands', resultType: 'command' },
    { name: 'cargo build', subtitle: 'Compile Rust project', category: 'commands', resultType: 'command' },
  ]
  const all = [...allApps, ...allFiles, ...allCommands]
  return all
    .filter((s) => !query.trim() || s.name.toLowerCase().includes(lower) || s.subtitle.toLowerCase().includes(lower))
    .map((s, idx) => ({
      id: String(idx),
      title: s.name,
      subtitle: s.subtitle,
      icon: null,
      category: s.category,
      resultType: (s as any).resultType || 'other-file',
      action: { type: 'launch', data: s.subtitle },
      score: 1 - idx * 0.01,
    }))
}
