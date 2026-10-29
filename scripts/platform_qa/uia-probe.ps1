# Diagnostic: find out what the packaged window exposes to UI Automation and
# what it takes to activate the tree. Not part of the acceptance run; the
# journey harness consumes whatever this establishes.
[CmdletBinding()]
param(
    [string]$Exe = 'aperture.exe',
    [string]$Journey = '.',
    [string]$Out = 'uia-probe',
    [string[]]$AppArgs = @('--seed', '4242', '--seats', '2', '--size', '1280x720')
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

if (-not ('ApertureQa.Win32' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
namespace ApertureQa {
    public static class Win32 {
        [DllImport("user32.dll", SetLastError=true)]
        public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
        [DllImport("user32.dll")]
        public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
        [DllImport("user32.dll")]
        public static extern bool IsWindow(IntPtr hWnd);
        [StructLayout(LayoutKind.Sequential)]
        public struct RECT { public int Left, Top, Right, Bottom; }
    }
}
'@
}

if (-not (Test-Path $Out)) { New-Item -ItemType Directory -Force -Path $Out | Out-Null }
$Exe = (Resolve-Path $Exe).Path
$Journey = (Resolve-Path $Journey).Path

function Count-Tree([IntPtr]$Hwnd) {
    try {
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($Hwnd)
        if ($null -eq $root) { return -1 }
        $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition)
        return $all.Count
    } catch {
        Write-Host ("  query error: {0}" -f $_.Exception.Message)
        return -2
    }
}

function Get-Names([IntPtr]$Hwnd, [int]$Max = 40) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($Hwnd)
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition)
    $out = New-Object System.Collections.ArrayList
    foreach ($el in $all) {
        if ($out.Count -ge $Max) { break }
        $c = $el.Current
        [void]$out.Add(('{0} | {1} | enabled={2}' -f $c.ControlType.ProgrammaticName, $c.Name, $c.IsEnabled))
    }
    return $out
}

$argumentLine = ($AppArgs | ForEach-Object { if ($_ -match '\s') { '"' + $_ + '"' } else { $_ } }) -join ' '
Write-Host ("launch: {0} {1} (cwd {2})" -f $Exe, $argumentLine, $Journey)
$proc = Start-Process -FilePath $Exe -ArgumentList $argumentLine -WorkingDirectory $Journey -PassThru
$deadline = (Get-Date).AddSeconds(30)
$hwnd = [IntPtr]::Zero
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 250
    $proc.Refresh()
    if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { $hwnd = $proc.MainWindowHandle; break }
}
Write-Host ("hwnd={0} pid={1}" -f $hwnd, $proc.Id)
Start-Sleep -Milliseconds 1500

# Attempt 1: straight from the window handle.
$count = Count-Tree $hwnd
Write-Host ("A0 handle-descendants: {0}" -f $count)

# Attempt 2: from the desktop root, by process id.
try {
    $cond = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $proc.Id)
    $desktop = [System.Windows.Automation.AutomationElement]::RootElement
    $window = $desktop.FindFirst([System.Windows.Automation.TreeScope]::Children, $cond)
    if ($null -ne $window) {
        Write-Host ("A1 desktop child found: '{0}' class={1}" -f $window.Current.Name, $window.Current.ClassName)
        $sub = $window.FindAll([System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition)
        Write-Host ("A1 desktop-child descendants: {0}" -f $sub.Count)
    } else {
        Write-Host 'A1 no desktop child for that pid'
    }
} catch {
    Write-Host ("A1 error: {0}" -f $_.Exception.Message)
}

# Attempt 3: poke the window with a harmless message so egui runs a frame, then re-query.
for ($i = 1; $i -le 8; $i++) {
    [void][ApertureQa.Win32]::PostMessage($hwnd, 0x0200, [IntPtr]0, [IntPtr]0)
    Start-Sleep -Milliseconds 350
    $count = Count-Tree $hwnd
    Write-Host ("B{0} after mouse-move poke: {1}" -f $i, $count)
    if ($count -gt 0) { break }
}

# Attempt 4: keyboard poke (a key egui definitely handles: HELP = H) then re-query.
if ((Count-Tree $hwnd) -le 0) {
    [void][ApertureQa.Win32]::PostMessage($hwnd, 0x0100, [IntPtr]0x48, [IntPtr]1)
    Start-Sleep -Milliseconds 80
    [void][ApertureQa.Win32]::PostMessage($hwnd, 0x0101, [IntPtr]0x48, [IntPtr]0xC0000001)
    Start-Sleep -Milliseconds 600
    Write-Host ("C0 after key poke: {0}" -f (Count-Tree $hwnd))
}

$all = @()
try { $all = Get-Names $hwnd 60 } catch { }
Write-Host ("names: {0}" -f $all.Count)
$all | ForEach-Object { Write-Host ("  {0}" -f $_) }

$rect = New-Object ApertureQa.Win32+RECT
[void][ApertureQa.Win32]::GetWindowRect($hwnd, [ref]$rect)
Write-Host ("window rect: {0},{1} {2}x{3}" -f $rect.Left, $rect.Top, ($rect.Right - $rect.Left), ($rect.Bottom - $rect.Top))

try { $proc.CloseMainWindow() | Out-Null; Start-Sleep -Milliseconds 400 } catch { }
$proc.Refresh()
if (-not $proc.HasExited) { $proc.Kill() }
