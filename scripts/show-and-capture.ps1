Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

# #problems_and_diagnostics
#
# 该脚本在调试 MonoTools UI 时频繁踩坑, 这里集中记录 "问题 + 根因 + 规避" 三段式.
# 每次踩坑后必须追加, 防止 reverse drift; 配合 CLAUDE.md 编码规范 4.3 (console / 调试日志清理).
#
# P0 - 已知真实问题 (脚本当前未解决, 调用方需知)
# --------------------------------------------------------------------------------
# P0-1 [race-condition] EnumWindows 回调里写 $script:hwnd, 但 PS 闭包内修改 script-scope
#       变量在不同 PS 版本行为不一致. PS 5.1 (Desktop) 下能用, PS 7+ (Core) 下回调函数
#       默认在函数 scope, 必须用 [scriptblock]::Create() 包装 + & $cb.Invoke(...) 才能
#       看到外层变量. 现象: 脚本在 PowerShell 7 下永远返回 "MonoTools window not found".
#       规避: 本脚本仅在 Windows PowerShell 5.1 下跑; 在 pwsh 7 下用 `& $env:SystemRoot\
#       System32\WindowsPowerShell\v1.0\powershell.exe -File show-and-capture.ps1`.
#
# P0-2 [hardcode] L72 窗口尺寸 720x580 是历史 "0.5x 旧尺寸", 与 `config::ui::WINDOW_*
#       DIMENSIONS` (640x580) 不一致. 截图实际拿到的窗口大小可能比 store 期望的尺寸
#       大 80px, 边距 40px 出现在左侧. 调试前必须比对 tauri.conf.json 实际配置, 不要
#       默认 "脚本就是标准".
#
# P0-3 [no-retry] 脚本一次性查找 + 一次性截图. 若 WebView 还没初始化完 (启动 < 5s) 截到
#       的是空白 WebView 框. `Start-Sleep -Seconds 5` 是经验值, Windows 11 + 慢盘 +
#       大字体环境下可能需要 8s+. 规避: 加 retry / 等待 `indexed_apps > 0` 事件, 或
#       接受 "首次截图可能空白, 再跑一次".
#
# P1 - 待优化 (非紧急, 但应跟进)
# --------------------------------------------------------------------------------
# P1-1 [hardcode-path] L87 输出路径 `d:\Work\Code\MonoStudio\MonoTools\ui-screenshot.
#       png` 写死. CI / 其他开发者克隆到不同盘符时会失败. 计划抽到 `scripts/_config.ps1`
#       或接受 `-OutPath` 参数.
#
# P1-2 [no-error-context] "MonoTools window not found" 失败时没 dump 进程列表 + 所有
#       可见窗口标题, 排查时需要手动开 Get-Process / Get-Window 加 grep. 计划: 失败
#       分支写一段诊断输出.
#
# P1-3 [enum-fragile] 窗口标题用字符串相等 `"MonoTools"` 匹配, 一旦 tauri.conf.json
#       的 title 改了 (多语言 / 加版本号) 就匹配不上. 计划: 改用 `proc.MainWindowTitle`
#       一次拿主窗口, 或者只匹配 "开头是 MonoTools".
#
# P2 - 一次性 stub (可接受现状)
# --------------------------------------------------------------------------------
# P2-1 [here-string] Win32 P/Invoke 用 here-string 注入, 不能跨进程复用. 接受: 这是
#       单文件调试脚本, 不要求模块化.
#
# P2-2 [coord-hardcode] `SetWindowPos(..., 500, 250, ...)` 是手工挑的 "屏幕中央略偏".
#       多显示器环境会跑到错误屏幕. 接受: 调试脚本, 自己调整即可.

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class Win32 {
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int w, int h, uint flags);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

# 找到 monotools 的 MonoTools 窗口
$handleScript = @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public class W {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr lParam);
    [DllImport("user32.dll")] public static extern int GetWindowThreadProcessId(IntPtr hWnd, out int processId);
    [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
}
"@
Add-Type -TypeDefinition $handleScript

$proc = Get-Process monotools -ErrorAction SilentlyContinue | Select-Object -First 1
if ($null -eq $proc) {
    Write-Host "No monotools running"
    exit 1
}

# 枚举所有 monotools 窗口
$hwnd = [IntPtr]::Zero
$cb = [W+EnumWindowsProc]{
    param($h, $l)
    $procId = 0
    [W]::GetWindowThreadProcessId($h, [ref]$procId) | Out-Null
    if ($procId -eq $proc.Id) {
        $len = [W]::GetWindowTextLength($h)
        if ($len -gt 0) {
            $sb = New-Object System.Text.StringBuilder ($len + 1)
            [W]::GetWindowText($h, $sb, $sb.Capacity) | Out-Null
            $t = $sb.ToString()
            if ($t -eq "MonoTools") {
                $script:hwnd = $h
                return $false
            }
        }
    }
    return $true
}
[W]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null

if ($hwnd -eq [IntPtr]::Zero) {
    Write-Host "MonoTools window not found"
    exit 1
}

Write-Host "Found MonoTools HWND: $hwnd"

# ShowWindow SW_SHOWNOACTIVATE = 4
[Win32]::ShowWindow($hwnd, 5) | Out-Null
[Win32]::SetForegroundWindow($hwnd) | Out-Null
[Win32]::SetWindowPos($hwnd, [IntPtr]::Zero, 500, 250, 720, 580, 0x40) | Out-Null

# 等待 webview 渲染
Write-Host "Waiting 5s for WebView to render..."
Start-Sleep -Seconds 5

$rect = New-Object Win32+RECT
[Win32]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top
Write-Host "Window: ${width}x${height} at ($($rect.Left),$($rect.Top))"

$bmp = New-Object System.Drawing.Bitmap $width, $height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($rect.Left, $rect.Top, 0, 0, (New-Object System.Drawing.Size($width, $height)))
$bmp.Save("d:\Work\Code\MonoStudio\MonoTools\ui-screenshot.png", [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
Write-Host "Screenshot saved"
