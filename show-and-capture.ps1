Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

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
