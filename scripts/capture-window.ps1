Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

# #problems_and_diagnostics
#
# 简版截图脚本 (无坐标 / 无窗口定位), 跟 `scripts/show-and-capture.ps1` 是同源不同
# 程度. 集中记录踩过的坑, 防止 reverse drift. 配合 CLAUDE.md 编码规范 4.3.
#
# P0 - 已知真实问题
# --------------------------------------------------------------------------------
# P0-1 [no-window-title-fallback] L5 用 `MainWindowTitle -ne ""` 过滤, 但 monotools
#       启动初期 (< 500ms) MainWindowTitle 是空字符串, 脚本直接 "No monotools window
#       found" 退出. 规避: 跑脚本前先 sleep 2s 等窗口稳定, 或重试 3 次.
#
# P0-2 [no-rect-retry] L33 GetWindowRect 一次性, 若窗口在 GetWindowRect 和
#       CopyFromScreen 之间移动 / 最小化, 截到的是上一帧位置 (黑边 / 残影). 规避:
#       不在用户操作窗口时跑; CI / 自动测试场景才安全.
#
# P1 - 待优化
# --------------------------------------------------------------------------------
# P1-1 [hardcode-path] L43 输出路径 `d:\Work\Code\MonoStudio\MonoTools\ui-screenshot.
#       png` 写死, 与 `scripts/show-and-capture.ps1` 同样的问题. 计划: 抽到 `_config.
#       ps1` 共用.
#
# P1-2 [sw-restore] L28 ShowWindow(.., 9) 强制 SW_RESTORE, 若窗口已最大化反而把它
#       缩小. 同 `show-and-capture.ps1` P0-3: 接受 "启动后用户没动过窗口" 这一假设.
#
# P2 - 一次性 stub
# --------------------------------------------------------------------------------
# P2-1 [no-resize] 不调 SetWindowPos, 截图就是当前真实大小. 接受: 这是 "看现状" 脚本,
#       不是 "规范化截图" 脚本. 规范化用 show-and-capture.ps1.

# 获取 monotools 窗口
$proc = Get-Process monotools -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowTitle -ne "" } | Select-Object -First 1
if ($null -eq $proc) {
    Write-Host "No monotools window found"
    exit 1
}

# 显示窗口
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32 {
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

$hwnd = $proc.MainWindowHandle
[Win32]::ShowWindow($hwnd, 9) | Out-Null  # SW_RESTORE
[Win32]::SetForegroundWindow($hwnd) | Out-Null

Start-Sleep -Milliseconds 800

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
Write-Host "Saved screenshot to ui-screenshot.png"
