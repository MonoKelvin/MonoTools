/**
 * Native module stubs - replaces C++ native module with JavaScript/TypeScript implementations
 * Some features are stubbed/no-op, others use alternative approaches where possible
 */

// ========== WindowManager Stubs ==========

export class WindowManager {
  /**
   * 激活窗口（通过进程ID）
   * Windows: Uses PowerShell to bring window to foreground
   * Linux/macOS: No-op (platform limitation)
   */
  static activateWindow(pid: number): void {
    if (process.platform === 'win32') {
      try {
        // PowerShell script to bring process window to foreground
        const { execSync } = require('child_process')
        execSync(
          `powershell -Command "Add-Type @'\nusing System;\nusing System.Runtime.InteropServices;\npublic class Win32 {\n    [DllImport("user32.dll")]\n    public static extern bool SetForegroundWindow(IntPtr hWnd);\n}\n'@; $proc = Get-Process -Id ${pid} -ErrorAction SilentlyContinue; if ($proc) { [Win32]::SetForegroundWindow($proc.MainWindowHandle) }"`,
          { stdio: 'ignore' }
        )
      } catch (error) {
        console.warn('[WindowManager] Failed to activate window:', error)
      }
    } else {
      // Linux/macOS: Not implemented without native module
      console.log('[WindowManager] activateWindow not implemented on this platform')
    }
  }

  /**
   * 激活窗口（通过窗口ID，Linux-specific）
   * Uses wmctrl or xdotool
   */
  static activateWindowByWid(wid: string): void {
    if (process.platform === 'linux') {
      try {
        const { execSync } = require('child_process')
        // Try wmctrl first, fallback to xdotool
        try {
          execSync(`wmctrl -i -r ${wid} -b add,above`, { stdio: 'ignore' })
          execSync(`wmctrl -i -a ${wid}`, { stdio: 'ignore' })
        } catch {
          execSync(`xdotool windowactivate ${wid}`, { stdio: 'ignore' })
        }
      } catch (error) {
        console.warn('[WindowManager] Failed to activate window by WID:', error)
      }
    }
  }

  /**
   * 获取资源管理器当前文件夹路径（Windows-specific）
   * Uses COM automation via PowerShell
   */
  static getExplorerFolderPath(hwnd: number): string | null {
    if (process.platform !== 'win32' || hwnd <= 0) {
      return null
    }

    try {
      const { execSync } = require('child_process')
      // Use PowerShell and Shell.Application COM object to get Explorer folder path
      const script = `$shell = New-Object -ComObject Shell.Application; $window = $shell.Windows() | Where-Object { $_.HWND -eq ${hwnd} }; if ($window) { $window.Document.Folder.Self.Path } else { "" }`
      const result = execSync(`powershell -Command "${script}"`, {
        encoding: 'utf-8',
        timeout: 2000
      })
      const path = result.trim()
      return path || null
    } catch (error) {
      console.warn('[WindowManager] Failed to get Explorer folder path:', error)
      return null
    }
  }

  /**
   * 获取当前活动窗口信息
   * Returns basic info, HWND is platform-specific
   */
  static getActiveWindow(): { title?: string; app?: string; hwnd?: number } | null {
    if (process.platform === 'win32') {
      try {
        const { execSync } = require('child_process')
        // Use simpler PowerShell approach without C# Add-Type
        const psCommand = `$hwnd = (Get-Process -Id $PID).MainWindowHandle; if ($hwnd -eq 0) { $hwnd = [IntPtr]::Zero } else { $hwnd = [IntPtr]$hwnd }; Add-Type -AssemblyName System.Windows.Forms; $title = [System.Windows.Forms.Form]::ActiveForm?.Text; if (-not $title) { $title = "" }; $proc = Get-Process -Id (Get-Process -Id $PID).Parent.Id -ErrorAction SilentlyContinue; $appName = $proc.ProcessName; ConvertTo-Json -Compress @{ hwnd = $hwnd.ToInt64(); title = $title; app = $appName }`
        const result = execSync(`powershell -Command "${psCommand}"`, {
          encoding: 'utf-8',
          timeout: 2000
        })
        const parsed = JSON.parse(result.trim())
        return {
          hwnd: parsed.hwnd,
          title: parsed.title,
          app: parsed.app
        }
      } catch (error) {
        console.warn('[WindowManager] Failed to get active window:', error)
        return null
      }
    } else if (process.platform === 'darwin') {
      // macOS: Use AppleScript to get frontmost app
      try {
        const { execSync } = require('child_process')
        const appName = execSync(
          'osascript -e \'tell application "System Events" to get name of first application process whose frontmost is true\'',
          { encoding: 'utf-8' }
        ).trim()
        return { app: appName }
      } catch {
        return null
      }
    } else {
      // Linux: xdotool or xprop
      try {
        const { execSync } = require('child_process')
        const title = execSync('xdotool getwindowfocus getwindowname', { encoding: 'utf-8' }).trim()
        return { title }
      } catch {
        return null
      }
    }
  }

  /**
   * 获取所有资源管理器/访达窗口
   * Windows: PowerShell, macOS: AppleScript, Linux: not supported
   */
  static getAllExplorerWindows(): Array<{ hwnd?: number; title?: string; path?: string }> {
    if (process.platform === 'win32') {
      try {
        const { execSync } = require('child_process')
        const psScript = `$shell = New-Object -ComObject Shell.Application; $shell.Windows() | ForEach-Object { [PSCustomObject]@{ hwnd = $_.HWND; title = $_.Document.Title; path = $_.Document.Folder.Self.Path } } | ConvertTo-Json`
        const result = execSync(`powershell -Command "${psScript}"`, {
          encoding: 'utf-8',
          timeout: 3000
        })
        return JSON.parse(result.trim())
      } catch (error) {
        console.warn('[WindowManager] Failed to get all explorer windows:', error)
        return []
      }
    } else if (process.platform === 'darwin') {
      try {
        const { execSync } = require('child_process')
        const script = `
          tell application "Finder"
            set windowCount to count of windows
            set output to ""
            repeat with i from 1 to windowCount
              set windowPath to POSIX path of (target of front window as alias)
              set output to output & windowPath & linefeed
            end repeat
            return output
          end tell
        `
        const result = execSync(`osascript -e '${script.replace(/\n/g, '; ')}'`, {
          encoding: 'utf-8',
          timeout: 2000
        })
        const paths = result.trim().split('\n').filter(Boolean)
        return paths.map((path, index) => ({ title: `Finder ${index + 1}`, path }))
      } catch {
        return []
      }
    }
    return []
  }

  /**
   * 判断是否为文件管理器窗口
   */
  static isFileLocationWindow(hwnd: number): boolean {
    if (process.platform === 'win32' && hwnd > 0) {
      try {
        const { execSync } = require('child_process')
        const psScript = `$shell = New-Object -ComObject Shell.Application; $shell.Windows() | Where-Object { $_.HWND -eq ${hwnd} } | Measure-Object | Select-Object -ExpandProperty Count`
        const count = parseInt(
          execSync(`powershell -Command "${psScript}"`, { encoding: 'utf-8' }).trim()
        )
        return count > 0
      } catch {
        return false
      }
    }
    return false
  }

  /**
   * 设置文件管理器地址栏
   * Windows: PowerShell COM, macOS: AppleScript, Linux: not supported
   */
  static setAddressBar(target: number | string | any, address: string): boolean {
    if (process.platform === 'win32') {
      try {
        const { execSync } = require('child_process')
        const hwnd = typeof target === 'number' ? target : target?.hwnd
        if (!hwnd) return false

        const psScript = `$shell = New-Object -ComObject Shell.Application; $window = $shell.Windows() | Where-Object { $_.HWND -eq ${hwnd} }; if ($window) { $window.Document.Shell.UI.Item(1).InvokeVerb("Properties"); $true } else { $false }`
        execSync(`powershell -Command "${psScript}"`, { stdio: 'ignore' })
        return true
      } catch (error) {
        console.warn('[WindowManager] Failed to set address bar:', error)
        return false
      }
    } else if (process.platform === 'darwin') {
      try {
        const { execSync } = require('child_process')
        // macOS: Use AppleScript to set Finder location
        execSync(
          `osascript -e 'tell application "Finder" to set target of front window to (POSIX file "${address}" as alias)'`,
          { stdio: 'ignore', timeout: 2000 }
        )
        return true
      } catch {
        return false
      }
    }
    return false
  }

  /**
   * 模拟键盘输入
   * Uses Robot.js if available, otherwise platform-specific commands
   */
  static simulateKeyboardTap(
    key: string,
    modifier: 'ctrl' | 'meta' | 'alt' | 'shift' = 'ctrl'
  ): void {
    if (process.platform === 'win32') {
      try {
        // Try PowerShell with WScript.Shell
        const { execSync } = require('child_process')
        const modifierKey = modifier === 'meta' ? 'CTRL' : modifier.toUpperCase()
        execSync(
          `powershell -Command "$wshell = New-Object -ComObject WScript.Shell; $wshell.SendKeys('^${key.toUpperCase()}')"`,
          { stdio: 'ignore', timeout: 1000 }
        )
        return
      } catch {
        // Fallback to PowerShell sendkeys
      }
    } else if (process.platform === 'darwin') {
      try {
        const { execSync } = require('child_process')
        const modifierKey = modifier === 'meta' ? 'command' : modifier
        execSync(
          `osascript -e 'tell application "System Events" to keystroke "${key}" using ${modifierKey} down'`,
          {
            stdio: 'ignore',
            timeout: 1000
          }
        )
        return
      } catch {
        // Fallback
      }
    } else if (process.platform === 'linux') {
      try {
        const { execSync } = require('child_process')
        const modifierKey = modifier === 'meta' ? 'ctrl' : modifier
        execSync(`xdotool key --clearmodifiers ${modifierKey}+${key}`, {
          stdio: 'ignore',
          timeout: 1000
        })
        return
      } catch {
        // Fallback
      }
    }

    console.warn('[WindowManager] Keyboard simulation not available')
  }
}

// ========== MouseMonitor Stub ==========

export class MouseMonitor {
  static isMonitoring = false
  private static callback: (() => MouseMonitorResult) | null = null
  private static timer: NodeJS.Timeout | null = null
  private static longPressTimer: NodeJS.Timeout | null = null
  private static triggerButton: string = ''
  private static longPressMs: number = 0
  private static startTime: number = 0
  private static isPressed = false

  static start(
    button: 'left' | 'right' | 'middle' | 'back' | 'forward',
    longPressMs: number,
    callback: () => MouseMonitorResult
  ): void {
    if (this.isMonitoring) {
      this.stop()
    }

    this.triggerButton = button
    this.longPressMs = longPressMs
    this.callback = callback
    this.isMonitoring = true
    this.isPressed = false

    // Note: Without native uiohook, we cannot reliably detect mouse button presses
    // This is a limited implementation using polling/intervals
    console.log('[MouseMonitor] Started (limited implementation - native module not available)')

    // Polling fallback: check if mouse button state changed
    // This is NOT reliable but allows basic functionality
    this.startTime = Date.now()
  }

  static stop(): void {
    this.isMonitoring = false
    this.callback = null
    if (this.timer) {
      clearInterval(this.timer)
      this.timer = null
    }
    if (this.longPressTimer) {
      clearTimeout(this.longPressTimer)
      this.longPressTimer = null
    }
    console.log('[MouseMonitor] Stopped')
  }
}

export interface MouseMonitorResult {
  shouldBlock: boolean
}

// ========== ClipboardMonitor Stub ==========

export class ClipboardMonitorClass {
  private callback: (() => void) | null = null
  private timer: NodeJS.Timeout | null = null

  start(callback: () => void): void {
    this.callback = callback
    console.log('[ClipboardMonitor] Started (stub - actual monitoring handled by clipboardManager)')
  }

  stop(): void {
    this.callback = null
    if (this.timer) {
      clearInterval(this.timer)
      this.timer = null
    }
    console.log('[ClipboardMonitor] Stopped')
  }

  /**
   * 设置剪贴板轮询增强
   * In the JS implementation, this is handled by the clipboardManager
   * This stub is here for API compatibility
   */
  static setClipboardPollingBoost(_interval: number, _duration: number): void {
    console.log('[ClipboardMonitor] Polling boost requested (handled by JS clipboardManager)')
  }
}

// Export as both default and named for compatibility
export const ClipboardMonitor = ClipboardMonitorClass

// ========== WindowMonitor Stub ==========

export class WindowMonitorClass {
  constructor() {
    console.log('[WindowMonitor] Initialized (stub)')
  }

  start(): void {
    console.log('[WindowMonitor] start() called (stub - not implemented)')
  }

  stop(): void {
    console.log('[WindowMonitor] stop() called (stub - not implemented)')
  }
}

export const WindowMonitor = WindowMonitorClass

// Default export for compatibility with mixed imports
export default ClipboardMonitorClass

// ========== UwpManager Stub ==========

export class UwpManager {
  /**
   * 获取所有 UWP 应用
   * Uses PowerShell to enumerate installed UWP apps
   */
  static getUwpApps(): Array<{ name: string; appId: string; icon?: string }> {
    if (process.platform !== 'win32') {
      return []
    }

    try {
      const { execSync } = require('child_process')
      // Single-line PowerShell command to avoid syntax issues
      const psCommand = `Get-ChildItem "HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Appx\\AppxAllUserStore\\Applications" | ForEach-Object { $appId = $_.PSChildName; $displayName = (Get-ItemProperty "HKLM:\\SOFTWARE\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\CurrentVersion\\AppContainer\\Mappings\\$appId" -ErrorAction SilentlyContinue).DisplayName; if (-not $displayName) { $displayName = $appId.Split('!')[0] }; [PSCustomObject]@{ appId = $appId; name = $displayName } } | ConvertTo-Json`

      const result = execSync(`powershell -Command "${psCommand}"`, {
        encoding: 'utf-8',
        timeout: 5000
      })

      if (!result.trim()) return []

      const apps = JSON.parse(result.trim())
      return apps.map((app: any) => ({
        name: app.name,
        appId: app.appId,
        icon: undefined // UWP icons require more complex extraction
      }))
    } catch (error) {
      console.warn('[UwpManager] Failed to get UWP apps:', error)
      return []
    }
  }

  /**
   * 启动 UWP 应用
   * Uses PowerShell to launch UWP apps via shell:AppsFolder
   */
  static launchUwpApp(appId: string): void {
    try {
      const { execSync } = require('child_process')
      // PowerShell command to launch UWP app
      execSync(`powershell -Command "explorer.exe shell:AppsFolder\\${appId}"`, {
        stdio: 'ignore',
        detached: true
      })
    } catch (error) {
      console.error('[UwpManager] Failed to launch UWP app:', error)
      throw error
    }
  }
}

// ========== ScreenCapture (moved from native) ==========
// ScreenCapture functionality has been moved to src/main/core/screenCapture.ts
// This stub is for API compatibility only
export class ScreenCapture {
  static async capture(): Promise<Buffer | null> {
    console.warn(
      '[ScreenCapture] Native module removed, use screenCapture from core/screenCapture.ts'
    )
    return null
  }
}

// ========== ColorPicker Stub ==========

export class ColorPicker {
  static async pickColor(): Promise<{ r: number; g: number; b: number; a: number } | null> {
    console.warn('[ColorPicker] Native module removed, color picker not available')
    return null
  }
}
