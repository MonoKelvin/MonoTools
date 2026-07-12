Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

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
