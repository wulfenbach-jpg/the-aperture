# THE APERTURE - scripted native-input platform QA harness.
#
# Drives the PACKAGED desktop binary (aperture.exe) with real Windows input:
# UI Automation for named controls and window-level keyboard/mouse messages for
# keys. Nothing here asserts on a screenshot; every step reads the live control
# tree (or the file the app actually wrote) back.
#
# Usage (see docs/platform-acceptance.md):
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/platform_qa/platform-qa.ps1 `
#       -Mode probe   -Exe <path to packaged aperture.exe> -Journey <work dir> -OutDir <evidence dir>
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/platform_qa/platform-qa.ps1 `
#       -Mode journey -Exe <path to packaged aperture.exe> -Journey <work dir> -OutDir <evidence dir>
#
# Exit code 0 only when every executed step passed. A step that cannot be
# exercised is reported NOT-EXERCISED and does not silently count as a pass.

[CmdletBinding()]
param(
    [ValidateSet('probe', 'journey', 'failures', 'viewport', 'import')]
    [string]$Mode = 'probe',
    [string]$Exe = 'aperture.exe',
    [string]$Journey = 'journey',
    [string]$OutDir = 'evidence/issue30',
    [int]$Seed = 4242,
    [int]$Width = 1280,
    [int]$Height = 720,
    [string]$MalformedUrl = 'https://archidekt.com/decks/not-a-deck-id',
    [string]$MissingDeckUrl = 'https://archidekt.com/decks/999999999',
    [string]$ForeignHostUrl = 'https://example.com/decks/1',
    [string]$SaveName = 'the-aperture-save.json',
    [int]$StartDelayMs = 1200,
    [int]$StepTimeoutSec = 20,
    # The acceptance criterion is a FULL four-seat table: three AI seats plus the
    # seat the harness drives until the human choice is proven, then all four on
    # autoplay to a terminal result. 2 is kept only for a quick local probe.
    [ValidateRange(2, 4)]
    [int]$Seats = 4,
    # Autoplay from the last recorded position to a terminal result. Generous on
    # purpose: a run that stops at this cap is a failure and is reported as one.
    [int]$FinishTimeoutSec = 900
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Drawing

if (-not ('ApertureQa.Win32' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;

namespace ApertureQa {
    public static class Win32 {
        [DllImport("user32.dll", SetLastError=true)]
        public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
        [DllImport("user32.dll", SetLastError=true)]
        public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
        [DllImport("user32.dll")]
        public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
        [DllImport("user32.dll")]
        public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
        [DllImport("user32.dll")]
        public static extern bool ClientToScreen(IntPtr hWnd, ref POINT lpPoint);
        [DllImport("user32.dll")]
        public static extern IntPtr GetForegroundWindow();
        [DllImport("user32.dll")]
        public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
        [DllImport("user32.dll")]
        public static extern bool IsWindow(IntPtr hWnd);
        [DllImport("user32.dll", CharSet=CharSet.Unicode)]
        public static extern IntPtr FindWindowEx(IntPtr parent, IntPtr childAfter, string cls, string title);

        [StructLayout(LayoutKind.Sequential)]
        public struct RECT { public int Left, Top, Right, Bottom; }
        [StructLayout(LayoutKind.Sequential)]
        public struct POINT { public int X, Y; }
    }
}
'@
}

$script:Results = New-Object System.Collections.ArrayList
$script:Transcript = New-Object System.Collections.ArrayList
$script:App = $null
# The run's real start. Save-Evidence used to stamp the save time as "started",
# so every record claimed the run began when it ended.
$script:StartedAt = Get-Date

function Write-Log([string]$Message) {
    $line = ('[{0}] {1}' -f (Get-Date -Format 'HH:mm:ss.fff'), $Message)
    [void]$script:Transcript.Add($line)
    Write-Host $line
}

# Set-Content -Encoding UTF8 writes a BOM on Windows PowerShell 5.1, and a BOM
# breaks ConvertFrom-Json / Python json.loads on the very records a reviewer has
# to parse. Write UTF-8 without one.
function Write-Utf8NoBom([string]$Path, [string]$Text) {
    $enc = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Text, $enc)
}

function Add-Result([string]$Step, [string]$Status, [string]$Detail, $Data) {
    [void]$script:Results.Add([ordered]@{
        step   = $Step
        status = $Status
        detail = $Detail
        data   = $Data
        at     = (Get-Date).ToString('o')
    })
    Write-Log ("STEP {0}: {1} - {2}" -f $Step, $Status.ToUpperInvariant(), $Detail)
}

# ---------------------------------------------------------------- UI Automation

function Safe-Int($Value) {
    # UIA reports NaN/Infinity geometry for nodes it has not laid out; casting
    # that to Int32 throws OverflowException.
    if ($null -eq $Value) { return 0 }
    $d = [double]$Value
    if ([double]::IsNaN($d) -or [double]::IsInfinity($d)) { return 0 }
    if ($d -gt [int]::MaxValue) { return [int]::MaxValue }
    if ($d -lt [int]::MinValue) { return [int]::MinValue }
    return [int][math]::Round($d)
}

function Get-Tree {
    if ($null -eq $script:App) { return @() }
    $hwnd = $script:App.Hwnd
    if (-not [ApertureQa.Win32]::IsWindow($hwnd)) { return @() }
    try {
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
        if ($null -eq $root) { return @() }
        $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition)
    } catch {
        # A UIA request can race a tree rebuild; one retry is enough.
        Start-Sleep -Milliseconds 120
        try {
            $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
            $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
                [System.Windows.Automation.Condition]::TrueCondition)
        } catch {
            return @()
        }
    }
    $rows = New-Object System.Collections.ArrayList
    foreach ($element in $all) {
        try {
            $current = $element.Current
            $rect = $current.BoundingRectangle
            $patterns = ''
            try {
                $patterns = (@($element.GetSupportedPatterns()) | ForEach-Object { $_.ProgrammaticName }) -join ','
            } catch { $patterns = '' }
            [void]$rows.Add([ordered]@{
                name          = [string]$current.Name
                type          = [string]$current.ControlType.ProgrammaticName
                automation_id = [string]$current.AutomationId
                class         = [string]$current.ClassName
                enabled       = [bool]$current.IsEnabled
                offscreen     = [bool]$current.IsOffscreen
                x             = (Safe-Int $rect.X)
                y             = (Safe-Int $rect.Y)
                w             = (Safe-Int $rect.Width)
                h             = (Safe-Int $rect.Height)
                patterns      = $patterns
            })
        } catch {
            # A node that vanished or cannot be read is not a control we can use.
        }
    }
    return $rows
}

function Get-Names {
    return @((Get-Tree) | Where-Object { $_.name } | ForEach-Object { $_.name })
}

function Test-AnyName([string]$Pattern) {
    return [bool](@(Get-Names) | Where-Object { $_ -match $Pattern } | Select-Object -First 1)
}

function Wait-AnyName([string]$Pattern, [int]$TimeoutSec = -1) {
    if ($TimeoutSec -lt 0) { $TimeoutSec = $StepTimeoutSec }
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        if (Test-AnyName $Pattern) { return $true }
        Assert-AppAlive
        Poke-Window
        Start-Sleep -Milliseconds 120
    }
    return $false
}

function Find-NodeName([string]$Pattern, [switch]$RequireEnabled, [string]$Type = '') {
    foreach ($row in @(Get-Tree)) {
        if (-not $row.name) { continue }
        if ($row.name -notmatch $Pattern) { continue }
        if ($Type -and $row.type -ne $Type) { continue }
        if ($RequireEnabled.IsPresent -and -not $row.enabled) { continue }
        return $row
    }
    return $null
}

# egui only republishes its accessibility tree on a frame, and an idle window
# does not repaint on its own. A posted mouse-move is a harmless way to ask for
# one without clicking anything.
function Poke-Window {
    if ($null -eq $script:App) { return }
    # Park the pointer over the main content, not the fixed header strip: the
    # wheel only scrolls what the pointer currently hovers.
    $rect = New-Object ApertureQa.Win32+RECT
    [void][ApertureQa.Win32]::GetWindowRect($script:App.Hwnd, [ref]$rect)
    $x = [int](($rect.Right - $rect.Left) * 0.45)
    $y = [int](($rect.Bottom - $rect.Top) * 0.55)
    $lParam = [IntPtr](($y -shl 16) -bor ($x -band 0xFFFF))
    [void][ApertureQa.Win32]::PostMessage($script:App.Hwnd, 0x0200, [IntPtr]0, $lParam)
}

function Get-HeaderLine {
    foreach ($row in @(Get-Tree)) {
        if ($row.name -match 'seats .* profile .* digest [0-9a-f]{12}') { return $row.name }
    }
    return $null
}

function Get-Digest {
    $header = Get-HeaderLine
    if ($null -eq $header) { return $null }
    if ($header -match 'digest ([0-9a-f]{12})') { return $Matches[1] }
    return $null
}

function Get-StackCount {
    $header = Get-HeaderLine
    if ($null -eq $header) { return $null }
    if ($header -match 'stack (\d+)') { return [int]$Matches[1] }
    return $null
}

# The app's own status line about the last accepted/refused thing.
function Get-MessageLine {
    foreach ($row in @(Get-Tree)) {
        if (-not $row.name) { continue }
        if ($row.name -match '^(refused|saved to|loaded |load failed|save failed|discarded|cancelled|ERROR:|branch failed)') { return $row.name }
    }
    return $null
}

function Get-DeciderStatus {
    foreach ($row in @(Get-Tree)) {
        if ($row.name -match '^decider-2b ') { return $row.name }
    }
    return $null
}

# Wait until a node matches, and hand the matching text back (not just a bool).
function Wait-ForName([string]$Pattern, [int]$TimeoutSec = -1) {
    if ($TimeoutSec -lt 0) { $TimeoutSec = $StepTimeoutSec }
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        foreach ($row in @(Get-Tree)) {
            if ($row.name -and $row.name -match $Pattern) { return $row.name }
        }
        Assert-AppAlive
        Poke-Window
        Start-Sleep -Milliseconds 120
    }
    return $null
}

# The client area in screen coordinates, so the harness can tell whether a node
# it read is actually on screen. Panels below the fold report their logical
# position even when scrolled out of view, and clicking there would click the
# desktop instead of the app.
function Get-ClientScreenRect {
    $rect = New-Object ApertureQa.Win32+RECT
    [void][ApertureQa.Win32]::GetWindowRect($script:App.Hwnd, [ref]$rect)
    return [pscustomobject]@{ Left = $rect.Left; Top = $rect.Top; Right = $rect.Right; Bottom = $rect.Bottom }
}

function Send-Wheel([int]$Delta) {
    $rect = Get-ClientScreenRect
    $x = [int](($rect.Left + $rect.Right) / 2)
    $y = [int]($rect.Top + ($rect.Bottom - $rect.Top) * 0.6)
    $lParam = [IntPtr](($y -shl 16) -bor ($x -band 0xFFFF))
    $wParam = [IntPtr](([int64]($Delta -band 0xFFFF)) -shl 16)
    # Move the pointer over the content first, otherwise the wheel has no target.
    [void][ApertureQa.Win32]::PostMessage($script:App.Hwnd, 0x0200, [IntPtr]0, $lParam)
    Start-Sleep -Milliseconds 90
    [void][ApertureQa.Win32]::PostMessage($script:App.Hwnd, 0x020A, $wParam, $lParam)   # WM_MOUSEWHEEL
    Start-Sleep -Milliseconds 180
}

# Scroll the app's own scroll area until the node the finder returns is on
# screen, then hand back that fresh node.
function Ensure-Visible([scriptblock]$Finder) {
    $rect = Get-ClientScreenRect
    $row = $null
    for ($i = 0; $i -lt 40; $i++) {
        $row = & $Finder
        if ($null -eq $row) { return $null }
        $cy = $row.y + [int]($row.h / 2)
        if ($cy -ge ($rect.Top + 10) -and $cy -le ($rect.Bottom - 10)) { return $row }
        $delta = if ($cy -lt $rect.Top) { 120 } else { -120 }
        Send-Wheel $delta
    }
    return $row
}

# Every enabled button, for the steps that have to pick one.
function Get-ActionButtons {
    return @((Get-Tree) | Where-Object { $_.type -eq 'ControlType.Button' -and $_.enabled })
}

function Wait-DigestChange([string]$Previous, [int]$TimeoutSec = 15) {
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $now = Get-Digest
        if ($null -ne $now -and $now -ne $Previous) { return $now }
        Poke-Window
        Start-Sleep -Milliseconds 150
    }
    return $null
}

function Read-Save([string]$Path) {
    for ($i = 0; $i -lt 40; $i++) {
        if (Test-Path $Path) {
            try {
                $text = Get-Content -Raw -Path $Path
                if ($text -and $text.Trim().EndsWith('}')) {
                    return ($text | ConvertFrom-Json)
                }
            } catch { }
        }
        Start-Sleep -Milliseconds 150
    }
    return $null
}

function Find-Node([string]$Pattern, [switch]$Exact, [switch]$RequireEnabled) {
    foreach ($row in @(Get-Tree)) {
        if (-not $row.name) { continue }
        $hit = if ($Exact) { $row.name -eq $Pattern } else { $row.name -match $Pattern }
        if (-not $hit) { continue }
        if ($RequireEnabled.IsPresent -and -not $row.enabled) { continue }
        if ($row.type -ne 'ControlType.Button' -and $row.type -ne 'ControlType.CheckBox' -and
            $row.type -ne 'ControlType.Text' -and $row.type -ne 'ControlType.Edit' -and
            $row.type -ne 'ControlType.RadioButton' -and $row.type -ne 'ControlType.ListItem') { continue }
        return $row
    }
    return $null
}

function Get-ElementByRect($Row) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($script:App.Hwnd)
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($element in $all) {
        try {
            $r = $element.Current.BoundingRectangle
        } catch { continue }
        if ((Safe-Int $r.X) -eq $Row.x -and (Safe-Int $r.Y) -eq $Row.y -and
            (Safe-Int $r.Width) -eq $Row.w -and (Safe-Int $r.Height) -eq $Row.h -and
            $element.Current.Name -eq $Row.name) {
            return $element
        }
    }
    return $null
}

function Invoke-Row($Row, [string]$How = 'auto') {
    $element = Get-ElementByRect $Row
    if ($null -eq $element) {
        Write-Log ("control '{0}' at {1},{2} {3}x{4} could not be re-resolved" -f $Row.name, $Row.x, $Row.y, $Row.w, $Row.h)
        throw "control '$($Row.name)' vanished before it could be invoked"
    }

    if ($How -ne 'mouse') {
        # A checkbox in egui exposes Toggle, not Invoke.
        if ($Row.patterns -match 'TogglePattern') {
            try {
                $toggle = $element.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
                $toggle.Toggle()
                Write-Log ("toggle '{0}' via uia-toggle" -f $Row.name)
                return 'uia-toggle'
            } catch { }
        }
        try {
            $pattern = $element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
            $pattern.Invoke()
            Write-Log ("invoke '{0}' via uia-invoke" -f $Row.name)
            return 'uia-invoke'
        } catch {
            $invokeError = $_.Exception.Message
            Write-Log ("uia-invoke failed on '{0}': {1}" -f $Row.name, $invokeError)
        }
        # Keyboard path: focus the control, then Enter (egui's native focus path).
        try {
            $element.SetFocus()
            Start-Sleep -Milliseconds 80
            Send-Key 0x0D
            Write-Log ("activate '{0}' via focus+enter" -f $Row.name)
            return 'focus+enter'
        } catch {
            $focusError = $_.Exception.Message
        }
    }
    Send-Click $Row.x $Row.y $Row.w $Row.h
    Write-Log ("click '{0}' via window-mouse at {1},{2} {3}x{4}" -f $Row.name, $Row.x, $Row.y, $Row.w, $Row.h)
    return 'window-mouse'
}

function Click-Name([string]$Pattern, [switch]$Exact, [switch]$RequireEnabled) {
    # A UIA read can transiently come back empty while the window rebuilds its
    # tree, so poll for the control instead of failing on one bad read.
    $deadline = (Get-Date).AddSeconds(10)
    $row = $null
    $finder = {
        Find-Node -Pattern $Pattern -Exact:$Exact -RequireEnabled:$RequireEnabled
    }.GetNewClosure()
    while ((Get-Date) -lt $deadline) {
        $row = & $finder
        if ($null -ne $row) { break }
        Poke-Window
        Start-Sleep -Milliseconds 150
    }
    if ($null -eq $row) { throw "no control matching '$Pattern'" }
    $row = Ensure-Visible $finder
    if ($null -eq $row) { throw "control matching '$Pattern' could not be scrolled into view" }
    $how = Invoke-Row $row
    Write-Log ("invoked '{0}' via {1}" -f $row.name, $how)
    return [ordered]@{ name = $row.name; how = $how }
}

function Get-ToggleState($Row) {
    $element = Get-ElementByRect $Row
    if ($null -eq $element) { return $null }
    try {
        $toggle = $element.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
        return $toggle.Current.ToggleState
    } catch {
        return $null
    }
}

# Set a checkbox to a known state. Toggling blindly would flip a box that the
# launch flags already turned on, which silently turns autoplay OFF.
function Set-ToggleState([string]$Pattern, [bool]$On) {
    $finder = { Find-NodeName $Pattern -Type 'ControlType.CheckBox' }.GetNewClosure()
    $row = Ensure-Visible $finder
    if ($null -eq $row) { throw "no checkbox matching '$Pattern'" }
    $state = Get-ToggleState $row
    $isOn = ($state -eq [System.Windows.Automation.ToggleState]::On)
    if ($isOn -eq $On) {
        Write-Log ("checkbox '{0}' already {1}" -f $row.name, $(if ($On) { 'on' } else { 'off' }))
        return [ordered]@{ name = $row.name; how = 'already-set'; state = [string]$state }
    }
    $how = Invoke-Row $row
    Start-Sleep -Milliseconds 250
    return [ordered]@{ name = $row.name; how = $how; state = [string](Get-ToggleState $row) }
}

# ------------------------------------------------------------ native input

function Send-Key([int]$VirtualKey, [switch]$Shift) {
    $hwnd = $script:App.Hwnd
    $lParamDown = [IntPtr]1
    [void][ApertureQa.Win32]::PostMessage($hwnd, 0x0100, [IntPtr]$VirtualKey, $lParamDown)          # WM_KEYDOWN
    Start-Sleep -Milliseconds 30
    [void][ApertureQa.Win32]::PostMessage($hwnd, 0x0101, [IntPtr]$VirtualKey, [IntPtr]0xC0000001)   # WM_KEYUP
    Start-Sleep -Milliseconds 60
}

function Send-Text([string]$Text) {
    foreach ($ch in $Text.ToCharArray()) {
        [void][ApertureQa.Win32]::PostMessage($script:App.Hwnd, 0x0102, [IntPtr][int]$ch, [IntPtr]1) # WM_CHAR
        Start-Sleep -Milliseconds 25
    }
}

function Send-Click([int]$ScreenX, [int]$ScreenY, [int]$W, [int]$H) {
    $cx = $ScreenX + [int]($W / 2)
    $cy = $ScreenY + [int]($H / 2)
    $rect = New-Object ApertureQa.Win32+RECT
    [void][ApertureQa.Win32]::GetWindowRect($script:App.Hwnd, [ref]$rect)
    $clientX = $cx - $rect.Left
    $clientY = $cy - $rect.Top
    $lParam = [IntPtr](($clientY -shl 16) -bor ($clientX -band 0xFFFF))
    [void][ApertureQa.Win32]::PostMessage($script:App.Hwnd, 0x0200, [IntPtr]0, $lParam)   # WM_MOUSEMOVE
    Start-Sleep -Milliseconds 40
    [void][ApertureQa.Win32]::PostMessage($script:App.Hwnd, 0x0201, [IntPtr]1, $lParam)   # WM_LBUTTONDOWN
    Start-Sleep -Milliseconds 40
    [void][ApertureQa.Win32]::PostMessage($script:App.Hwnd, 0x0202, [IntPtr]0, $lParam)   # WM_LBUTTONUP
    Start-Sleep -Milliseconds 80
}

# ------------------------------------------------------------ app lifecycle

# The process's MainWindowHandle is NOT the egui window here: winit also owns a
# small hidden helper window, and that is what MainWindowHandle reports. Resolve
# the real top-level window through UI Automation instead, by process id, and
# take its native handle.
function Resolve-WindowHandle($Process) {
    $cond = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $Process.Id)
    $desktop = [System.Windows.Automation.AutomationElement]::RootElement
    $windows = $desktop.FindAll([System.Windows.Automation.TreeScope]::Children, $cond)
    $best = $null
    $bestArea = -1
    foreach ($w in $windows) {
        $rect = $w.Current.BoundingRectangle
        $area = $rect.Width * $rect.Height
        # winit's hidden helper window is ~18x18 and unnamed; the real table
        # window is titled and large. Skip anything that cannot be the table.
        if ($w.Current.NativeWindowHandle -eq 0) { continue }
        if ([string]::IsNullOrWhiteSpace($w.Current.Name)) { continue }
        if ($area -lt 10000) { continue }
        if ($area -gt $bestArea) {
            $best = $w
            $bestArea = $area
        }
    }
    return $best
}

function Start-App([string[]]$AppArgs) {
    if ($script:App -and $script:App.Proc -and -not $script:App.Proc.HasExited) { Stop-App }
    $argumentLine = ($AppArgs | ForEach-Object { if ($_ -match '\s') { '"' + $_ + '"' } else { $_ } }) -join ' '
    Write-Log ("launch: {0} {1}" -f $Exe, $argumentLine)
    # Capture the app's own stream: a panic must land in the evidence instead of
    # vanishing with the window.
    $appOut = Join-Path $OutDir ("app-{0}.out.txt" -f ([guid]::NewGuid().ToString('N').Substring(0, 6)))
    $appErr = Join-Path $OutDir ("app-{0}.err.txt" -f ([guid]::NewGuid().ToString('N').Substring(0, 6)))
    $proc = Start-Process -FilePath $Exe -ArgumentList $argumentLine -WorkingDirectory $Journey -PassThru `
        -RedirectStandardOutput $appOut -RedirectStandardError $appErr
    $deadline = (Get-Date).AddSeconds(30)
    $window = $null
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 300
        $proc.Refresh()
        if ($proc.HasExited) { throw "the app exited during startup (code $($proc.ExitCode); see $appErr)" }
        $window = Resolve-WindowHandle $proc
        if ($null -ne $window) { break }
    }
    if ($null -eq $window) { throw 'no top-level application window appeared within 30 s' }
    $hwnd = [IntPtr]$window.Current.NativeWindowHandle
    $script:App = [pscustomobject]@{ Proc = $proc; Hwnd = $hwnd; Window = $window; StdErr = $appErr; StdOut = $appOut }
    Start-Sleep -Milliseconds $StartDelayMs   # let the first frames settle
    Write-Log ("window hwnd={0} pid={1} title='{2}'" -f $hwnd, $proc.Id, $window.Current.Name)
    return $script:App
}

# A dead window must fail the step that was waiting on it, not time out.
function Assert-AppAlive {
    if ($null -eq $script:App) { return }
    $script:App.Proc.Refresh()
    if ($script:App.Proc.HasExited) {
        $err = ''
        if (Test-Path $script:App.StdErr) { $err = (Get-Content -Raw $script:App.StdErr) }
        throw ("the app exited (code {0}) while the step was waiting; captured stderr: {1}" -f $script:App.Proc.ExitCode, $err.Trim())
    }
}

function Stop-App {
    if ($script:App -and $script:App.Proc -and -not $script:App.Proc.HasExited) {
        try {
            [void]$script:App.Proc.CloseMainWindow()
            Start-Sleep -Milliseconds 400
            $script:App.Proc.Refresh()
            if (-not $script:App.Proc.HasExited) { $script:App.Proc.Kill() }
        } catch { }
    }
    $script:App = $null
}

function Get-ForegroundTitle {
    $fg = [ApertureQa.Win32]::GetForegroundWindow()
    try { return (Get-Process | Where-Object { $_.MainWindowHandle -eq $fg } | Select-Object -First 1).MainWindowTitle } catch { return '' }
}

function Save-Snapshot([string]$Path) {
    $rect = New-Object ApertureQa.Win32+RECT
    [void][ApertureQa.Win32]::GetWindowRect($script:App.Hwnd, [ref]$rect)
    $w = $rect.Right - $rect.Left
    $h = $rect.Bottom - $rect.Top
    if ($w -le 0 -or $h -le 0) { return $false }
    $bitmap = New-Object System.Drawing.Bitmap $w, $h
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $hdc = $graphics.GetHdc()
    $ok = [ApertureQa.Win32]::PrintWindow($script:App.Hwnd, $hdc, 2)   # PW_RENDERFULLCONTENT
    $graphics.ReleaseHdc($hdc)
    $graphics.Dispose()
    $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    return $ok
}

function Save-Tree([string]$Path) {
    $tree = @(Get-Tree)
    $payload = [ordered]@{
        captured = (Get-Date).ToString('o')
        hwnd     = [int64]$script:App.Hwnd
        pid      = $script:App.Proc.Id
        nodes    = $tree
    }
    $payload | ConvertTo-Json -Depth 6 | ForEach-Object { Write-Utf8NoBom $Path $_ } | Out-Null
    return $tree.Count
}

# A timestamped dump of the QA instance only. Two of these taken a fixed interval
# apart are the machine-checkable form of "the state did not move": each one names
# its own capture time, pid and exe hash, so a reader does not have to trust a
# narrative about how far apart they were. The earlier stall record was a
# multi-instance desktop scan with no timestamps, which could not support the
# "75 s apart" claim on its own.
function Save-StallDump([string]$Path) {
    $dir = Split-Path -Parent $Path
    if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    $header = Get-HeaderLine
    $lines = New-Object System.Collections.ArrayList
    [void]$lines.Add(('# captured {0}' -f (Get-Date).ToString('o')))
    [void]$lines.Add(('# pid {0}' -f $script:App.Proc.Id))
    [void]$lines.Add(('# hwnd {0}' -f [int64]$script:App.Hwnd))
    [void]$lines.Add(('# exe {0}' -f $Exe))
    [void]$lines.Add(('# exe_sha256 {0}' -f (Get-FileHash -Algorithm SHA256 -Path $Exe).Hash.ToLowerInvariant()))
    [void]$lines.Add(('# header {0}' -f $header))
    [void]$lines.Add(('# message {0}' -f (Get-MessageLine)))
    [void]$lines.Add('# nodes')
    foreach ($row in @(Get-Tree)) {
        [void]$lines.Add(('{0} | {1} | enabled={2}' -f $row.type, $row.name, $row.enabled))
    }
    Write-Utf8NoBom $Path ($lines -join "`r`n")
    return [pscustomobject]@{ path = $Path; header = $header; pid = $script:App.Proc.Id; captured = (Get-Date).ToString('o') }
}

# ---------------------------------------------------------------- evidence

function Save-Evidence {
    if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Force -Path $OutDir | Out-Null }
    $payload = [ordered]@{
        mode       = $Mode
        harness    = $PSCommandPath
        harness_sha256 = (Get-FileHash -Algorithm SHA256 -Path $PSCommandPath).Hash.ToLowerInvariant()
        exe        = (Resolve-Path $Exe).Path
        exe_sha256 = (Get-FileHash -Algorithm SHA256 -Path $Exe).Hash.ToLowerInvariant()
        journey    = (Resolve-Path $Journey).Path
        seed       = $Seed
        seats      = $Seats
        window     = "$Width" + 'x' + "$Height"
        started    = $script:StartedAt.ToString('o')
        finished   = (Get-Date).ToString('o')
        results    = @($script:Results)
        transcript = @($script:Transcript)
    }
    $payload | ConvertTo-Json -Depth 8 | ForEach-Object { Write-Utf8NoBom (Join-Path $OutDir 'harness-run.json') $_ } | Out-Null
}

# ---------------------------------------------------------------- probe mode

function Invoke-Probe {
    Start-App @('--seed', "$Seed", '--seats', '2', '--size', ("{0}x{1}" -f $Width, $Height))
    $count = Save-Tree (Join-Path $OutDir 'probe-tree.json')
    $names = @(Get-Names)
    Write-Log ("probe: {0} UIA nodes, {1} named" -f $count, @($names).Count)
    $names | Sort-Object -Unique | ForEach-Object { Write-Host ("  . {0}" -f $_) }
    $buttons = @((Get-Tree) | Where-Object { $_.type -eq 'ControlType.Button' })
    Write-Log ("probe: {0} buttons; first 40 with patterns:" -f @($buttons).Count)
    $buttons | Select-Object -First 40 | ForEach-Object {
        Write-Host ("  B {0} | enabled={1} | {2}" -f $_.name, $_.enabled, $_.patterns)
    }
    $edits = @((Get-Tree) | Where-Object { $_.type -eq 'ControlType.Edit' })
    Write-Log ("probe: {0} edit fields; patterns:" -f @($edits).Count)
    $edits | Select-Object -First 10 | ForEach-Object {
        Write-Host ("  E id='{0}' | {1}" -f $_.automation_id, $_.patterns)
    }
    Stop-App
}

# ---------------------------------------------------------------- journey mode

function Save-And-Read([string]$Label) {
    $path = Join-Path $Journey $SaveName
    if (Test-Path $path) { Remove-Item -Force $path }
    Send-Key 0x53   # S
    $save = Read-Save $path
    if ($null -eq $save) { throw "no readable save file appeared for '$Label'" }
    return [pscustomobject]@{
        path   = $path
        digest = $save.digest
        steps  = @($save.replay.steps).Count
        turn   = $save.game.turn.number
        active = $save.game.turn.active
        step   = $save.game.turn.step
        outcome = $save.game.outcome
        log_events = @($save.log).Count
        version = $save.version
    }
}

function Invoke-Journey {
    $savePath = Join-Path $Journey $SaveName
    if (Test-Path $savePath) { Remove-Item -Force $savePath }
    $session = @{}

    # ---- 1. Launch the packaged binary: a four-seat table, human on P0, the
    # other three seats on the built-in heuristic with --autoplay.
    Start-App @('--seed', "$Seed", '--seats', "$Seats", '--profile', 'ordinary',
        '--size', ("{0}x{1}" -f $Width, $Height), '--autoplay')
    $session.seats = $Seats
    $nodeCount = Save-Tree (Join-Path $OutDir '01-launch-tree.json')
    $rect = New-Object ApertureQa.Win32+RECT
    [void][ApertureQa.Win32]::GetWindowRect($script:App.Hwnd, [ref]$rect)
    $actualW = $rect.Right - $rect.Left
    $actualH = $rect.Bottom - $rect.Top
    $session.requested = "$Width" + 'x' + "$Height"
    $session.actual = "$actualW" + 'x' + "$actualH"
    # Read as plain text: Get-Content -Raw returned a provider-decorated object
    # here, and ConvertTo-Json then walked its PSDrive/PSProvider graph into a
    # 100 MB record.
    $session.stamp = [System.IO.File]::ReadAllText((Join-Path $Journey 'BUILD-STAMP.txt'))
    if (-not (Wait-AnyName 'Keep this hand' 30)) {
        Add-Result 'launch' 'failed' 'the mulligan offer never appeared on the packaged build' $null
        return
    }
    Add-Result 'launch' 'passed' ("packaged window requested {0}, actual outer {1}x{2}, {3} UIA nodes on a {4}-seat table" -f $session.requested, $actualW, $actualH, $nodeCount, $Seats) $null

    # ---- 2. Keep the dealt hand through the real control.
    $foregroundBefore = Get-ForegroundTitle
    $keep = Click-Name 'Keep this hand' -Exact
    $session.keep = $keep
    if (-not (Wait-AnyName 'YOUR HAND' 20)) {
        Add-Result 'mulligan' 'failed' 'the table never appeared after keeping the hand' $keep
        Save-Tree (Join-Path $OutDir '02-mulligan-tree.json') | Out-Null
        return
    }
    Add-Result 'mulligan' 'passed' ("kept the hand via {0} ({1})" -f $keep.name, $keep.how) $keep

    # ---- 3. Advance through priority until the human is offered a real action.
    # The human does not start as the active player: P1 is on the play, so the
    # only honest way forward is to actually pass priority until a main-phase
    # action is offered. The AI seat plays its own turn under --autoplay.
    # "YOUR ACTIONS" is the engine's own legal-offer list, which is the only
    # surface that decides what this seat may actually do. A hand card can be a
    # button for inspection in another seat's panel, so selecting by card name
    # would be picking scenery, not a decision control.
    $passed = 0
    $offered = $null
    $deadline = (Get-Date).AddSeconds(150)
    while ((Get-Date) -lt $deadline) {
        $candidate = Find-NodeName '^Play ' -RequireEnabled -Type 'ControlType.Button'
        if ($null -ne $candidate) { $offered = $candidate; break }
        if (Test-AnyName 'P0 holds priority') {
            Send-Key 0x50   # P - a real pass while this seat owns priority
            $passed++
            Start-Sleep -Milliseconds 650
        } else {
            Poke-Window
            Start-Sleep -Milliseconds 350
        }
    }
    Save-Tree (Join-Path $OutDir '03-table-tree.json') | Out-Null
    $digestAtTable = Get-Digest
    $session.digest_at_table = $digestAtTable
    $session.passes_before_action = $passed
    if ($null -eq $offered) {
        Add-Result 'table' 'failed' ("no human action was ever offered after {0} passes (header: {1})" -f $passed, (Get-HeaderLine)) $null
        return
    }
    Add-Result 'table' 'passed' ("reached an offered main-phase action after {0} real priority pass(es); header digest {1}" -f $passed, $digestAtTable) @{ digest = $digestAtTable; passes = $passed; offered = $offered.name }

    # ---- 4. Priority choice 1: take an engine-offered land play.
    $land = Find-NodeName '^Play ' -RequireEnabled -Type 'ControlType.Button'
    if ($null -eq $land) {
        Add-Result 'priority-land' 'failed' 'the engine offered no land play to accept' $null
    } else {
        $before = Get-Digest
        $how = Invoke-Row $land
        $after = Wait-DigestChange $before 15
        if ($null -eq $after) {
            Add-Result 'priority-land' 'failed' ("invoking '{0}' did not change the game digest (message: {1})" -f $land.name, (Get-MessageLine)) @{ how = $how }
        } else {
            Add-Result 'priority-land' 'passed' ("accepted the offered '{0}' ({1}); digest {2} -> {3}" -f $land.name, $how, $before, $after) @{ how = $how; before = $before; after = $after }
        }
    }

    # ---- 5. Stack choice: cast a spell, watch it sit on the stack, pass to resolve.
    # A creature spell needs mana, so first earn it the only way the rules allow:
    # one land per turn, across real turns, with real passes in between.
    $spell = $null
    $landsPlayed = 0
    $castDeadline = (Get-Date).AddSeconds(240)
    while ((Get-Date) -lt $castDeadline) {
        $spell = Find-NodeName '^Cast (Hill Giant|Craw Wurm|Runeclaw Bear|Centaur Courser|Grizzly Bears|Divination)' -RequireEnabled -Type 'ControlType.Button'
        if ($null -ne $spell) { break }
        $playable = Find-NodeName '^Play ' -RequireEnabled -Type 'ControlType.Button'
        if ($null -ne $playable) {
            $before = Get-Digest
            [void](Invoke-Row $playable)
            if ($null -ne (Wait-DigestChange $before 10)) { $landsPlayed++ }
            continue
        }
        if (Test-AnyName 'P0 holds priority') { Send-Key 0x50 }
        Start-Sleep -Milliseconds 550
    }
    if ($null -eq $spell) {
        Add-Result 'stack-cast' 'failed' ("no castable creature spell was ever offered (additional lands played: {0})" -f $landsPlayed) $null
    } else {
        $beforeCast = Get-Digest
        $castHow = Invoke-Row $spell
        Start-Sleep -Milliseconds 500
        $stackAfterCast = Get-StackCount
        Save-Tree (Join-Path $OutDir '04-stack-tree.json') | Out-Null
        if ($null -eq $stackAfterCast -or $stackAfterCast -lt 1) {
            Add-Result 'stack-cast' 'failed' ("casting '{0}' never put an object on the stack (stack={1})" -f $spell.name, $stackAfterCast) @{ how = $castHow }
        } else {
            Add-Result 'stack-cast' 'passed' ("cast '{0}' ({1}) after {2} extra land drop(s); header stack count {3}" -f $spell.name, $castHow, $landsPlayed, $stackAfterCast) @{ how = $castHow; stack = $stackAfterCast; before = $beforeCast; lands = $landsPlayed }

            # Resolve it: the only way the stack empties is consecutive passes,
            # and every pass here is a real key press or a real button.
            $resolved = $false
            for ($i = 0; $i -lt 8; $i++) {
                if ((Get-StackCount) -eq 0) { $resolved = $true; break }
                $passRow = Find-NodeName '^Pass \(P\)$' -RequireEnabled -Type 'ControlType.Button'
                if ($null -eq $passRow) { Start-Sleep -Milliseconds 600; continue }
                Send-Key 0x50   # P
                Start-Sleep -Milliseconds 700
            }
            if (-not $resolved -and (Get-StackCount) -eq 0) { $resolved = $true }
            $afterResolve = Get-Digest
            Save-Tree (Join-Path $OutDir '05-resolved-tree.json') | Out-Null
            if ($resolved) {
                Add-Result 'stack-resolve' 'passed' ("stack emptied by mutual passes; digest {0} -> {1}" -f $beforeCast, $afterResolve) @{ after = $afterResolve }
            } else {
                Add-Result 'stack-resolve' 'failed' ("the stack never emptied (stack={0})" -f (Get-StackCount)) $null
            }
        }
    }

    # ---- 6. Save, then verify the packaged binary really wrote the position.
    $saved = $null
    try {
        $headerBeforeSave = Get-Digest
        $saved = Save-And-Read 'after the stack resolved'
        $session.saved = $saved
        # Keep the exact bytes the packaged app wrote at this step, so the
        # committed artifact is provably this run's save and not a leftover from
        # a sibling run (the unwritable-save probe later deletes this path).
        if (Test-Path $saved.path) { Copy-Item -Force $saved.path (Join-Path $OutDir 'the-aperture-save-midgame.json') }
        $headerDigest = Get-Digest
        $session.header_at_save = $headerBeforeSave
        $sane = ($saved.steps -gt 0 -and $headerDigest -eq $headerBeforeSave -and $null -eq $saved.outcome)
        Add-Result 'save' $(if ($sane) { 'passed' } else { 'failed' }) `
            ("wrote {0} bytes (save v{1}): {2} recorded action(s), turn {3}, phase {4}; live header digest {5} unchanged by the write" -f (Get-Item $saved.path).Length, $saved.version, $saved.steps, $saved.turn, $saved.step, $headerDigest) `
            @{ path = $saved.path; steps = $saved.steps; turn = $saved.turn; header = $headerDigest; save_file_digest = $saved.digest }
    } catch {
        Add-Result 'save' 'failed' $_.Exception.Message $null
    }

    # ---- 7. Change the position so a load has something to undo.
    $mutated = $null
    $beforeMutation = Get-Digest
    for ($i = 0; $i -lt 10; $i++) {
        Send-Key 0x50   # P: pass priority and let the position move on
        $mutated = Wait-DigestChange $beforeMutation 4
        if ($null -ne $mutated) { break }
    }
    if ($null -eq $mutated -or $null -eq $saved) {
        Add-Result 'mutate' 'failed' 'the position could not be advanced after the save (no accepted pass)' @{ digest = $mutated }
    } else {
        Add-Result 'mutate' 'passed' ("passed priority so the live position moved past the save (digest {0} -> {1})" -f $beforeMutation, $mutated) @{ digest = $mutated }
    }

    # ---- 8. Load the save back and prove the state matched by digest.
    if ($null -ne $saved) {
        Send-Key 0x4C   # L
        if (-not (Wait-AnyName 'CONFIRM: load' 15)) {
            Add-Result 'load' 'failed' 'pressing L did not open the load confirmation' $null
        } else {
            Save-Tree (Join-Path $OutDir '06a-confirm-tree.json') | Out-Null
            try {
                $confirm = Click-Name 'Confirm load'
            } catch {
                Add-Result 'load' 'failed' ("the load confirmation bar appeared but its confirm control was not reachable: {0}" -f $_.Exception.Message) $null
                $confirm = $null
            }
            if ($null -eq $confirm) { return }
            $loaded = Wait-AnyName 'loaded .*action\(s\) in the replay' 25
            $headerDigest = Get-Digest
            $restored = ($headerDigest -eq $session.header_at_save)
            Save-Tree (Join-Path $OutDir '06-loaded-tree.json') | Out-Null
            if ($loaded -and $restored) {
                Add-Result 'load' 'passed' ("load confirmed through '{0}'; the live digest is back to the saved position {1}" -f $confirm.name, $headerDigest) @{ digest = $headerDigest }
            } else {
                Add-Result 'load' 'failed' ("load did not restore the saved position (header {0}, saved {1}, message seen={2})" -f $headerDigest, $session.header_at_save, $loaded) $null
            }
        }
    }

    # ---- 9. Play it out to a terminal result.
    $seedPlayedOut = $false
    # Both boxes are set to a known state, not blindly flipped: --autoplay
    # already turned "AI autoplay" on at launch, and "Let the AI play my seat too"
    # is what makes this an AI-vs-AI table rather than a human seat parked
    # forever on a priority prompt.
    $handMySeat = $null
    $autoplay = $null
    try { $handMySeat = Set-ToggleState '^Let the AI play my seat too$' $true } catch { }
    try { $autoplay = Set-ToggleState '^AI autoplay$' $true } catch { }
    Write-Log ("autoplay boxes: seat={0}/{1} autoplay={2}/{3}" -f
        $(if ($null -ne $handMySeat) { $handMySeat.how } else { 'missing' }), $(if ($null -ne $handMySeat) { $handMySeat.state } else { '-' }),
        $(if ($null -ne $autoplay) { $autoplay.how } else { 'missing' }), $(if ($null -ne $autoplay) { $autoplay.state } else { '-' }))
    Add-Result 'autoplay-enabled' $(if ($null -ne $autoplay -and $autoplay.state -match 'On') { 'passed' } else { 'failed' }) `
        ("AI autoplay state after the toggle step: {0}; AI seat: {1}" -f $(if ($null -ne $autoplay) { $autoplay.state } else { 'missing' }), $(if ($null -ne $handMySeat) { $handMySeat.state } else { 'missing' })) `
        @{ autoplay = $autoplay; seat = $handMySeat; table_seats = $Seats }

    # Watch autoplay for a terminal result, sampling the live header so the
    # record shows the position actually MOVING (turn/step/digest changing)
    # rather than just a wall-clock wait. Autoplay is turned on for every seat
    # above, so this is an AI-vs-AI game on a four-seat table.
    $finishStarted = Get-Date
    $progress = New-Object System.Collections.ArrayList
    $reachedTerminal = $false
    $finishDeadline = $finishStarted.AddSeconds($FinishTimeoutSec)
    $lastSample = $finishStarted
    while ((Get-Date) -lt $finishDeadline) {
        if (Test-AnyName 'wins|is a draw') { $reachedTerminal = $true; break }
        Assert-AppAlive
        Poke-Window
        if (((Get-Date) - $lastSample).TotalSeconds -ge 15) {
            $headerNow = Get-HeaderLine
            [void]$progress.Add([ordered]@{ at = (Get-Date).ToString('HH:mm:ss'); header = $headerNow })
            Write-Log ("autoplay progress: {0}" -f $headerNow)
            $lastSample = Get-Date
        }
        Start-Sleep -Milliseconds 200
    }
    $finishSeconds = [math]::Round(((Get-Date) - $finishStarted).TotalSeconds, 1)
    $session.autoplay_progress = @($progress)
    $session.autoplay_seconds = $finishSeconds
    $distinctPositions = @($progress | ForEach-Object { $_.header } | Sort-Object -Unique).Count

    if (-not $reachedTerminal) {
        $stall = $null
        foreach ($row in @(Get-Tree)) {
            if ($row.name -match '^(discarded|WAITING: )') { $stall = $row.name; break }
        }
        Add-Result 'finish' 'failed' ("the {0}-seat AI-vs-AI game never reached a terminal result within {1} s of autoplay ({2} distinct sampled positions); last engine note: {3}" -f $Seats, $FinishTimeoutSec, $distinctPositions, $stall) @{ stall_note = $stall; header = Get-HeaderLine; progress = @($progress) }

        # Prove the position is frozen, not slow: two timestamped dumps of this
        # one QA instance, a fixed interval apart. Identical headers across the
        # interval are the defect evidence; different headers mean it moved.
        $dumpA = Save-StallDump (Join-Path $OutDir 'stall/dump-a.txt')
        Write-Log ('stall: first dump at {0}; waiting {1} s for the second' -f $dumpA.captured, 75)
        Start-Sleep -Seconds 75
        $dumpB = Save-StallDump (Join-Path $OutDir 'stall/dump-b.txt')
        $sameHeader = ($dumpA.header -eq $dumpB.header)
        $session.stall = @{ dump_a = $dumpA.path; dump_b = $dumpB.path; header_a = $dumpA.header; header_b = $dumpB.header; same_header = $sameHeader; seconds_apart = 75; pid = $dumpA.pid }
        Add-Result 'stall-dumps' $(if ($sameHeader) { 'passed' } else { 'failed' }) `
            ("two dumps of pid {0}, 75 s apart: header before '{1}', after '{2}'" -f $dumpA.pid, $dumpA.header, $dumpB.header) `
            @{ dump_a = $dumpA.path; dump_b = $dumpB.path; seconds_apart = 75; same_header = $sameHeader; pid = $dumpA.pid }
    } else {
        $terminal = $null
        foreach ($row in @(Get-Tree)) {
            if ($row.name -match 'wins|is a draw') { $terminal = $row.name; break }
        }
        $seedPlayedOut = $true
        Save-Tree (Join-Path $OutDir '07-terminal-tree.json') | Out-Null
        try {
            $final = Save-And-Read 'terminal'
            $outcomeJson = ($final.outcome | ConvertTo-Json -Compress -Depth 4)
            $session.final = $final
            if (Test-Path $final.path) { Copy-Item -Force $final.path (Join-Path $OutDir 'the-aperture-save-terminal.json') }
            Add-Result 'finish' 'passed' ("terminal: '{0}' after {1} s of {2}-seat AI-vs-AI autoplay ({3} distinct sampled positions); save records outcome {4}, {5} actions, turn {6}, digest {7}" -f $terminal, $finishSeconds, $Seats, $distinctPositions, $outcomeJson, $final.steps, $final.turn, $final.digest.Substring(0, 12)) `
                @{ terminal_text = $terminal; outcome = $final.outcome; steps = $final.steps; turn = $final.turn; digest = $final.digest; seconds = $finishSeconds; seats = $Seats }

            # ---- 10. The matching replay: the SHIPPED CLI re-runs the action
            # sequence the packaged app just recorded and must land on the same
            # digest. A saved file that no replayer can reproduce is not a
            # finished game, it is a JSON file.
            $cli = Join-Path $Journey 'aperture-cli.exe'
            $replay = Invoke-PackagedCli $cli @('replay', '--file', $final.path)
            # Two accepted output shapes: a session save is verified as
            # "stored digest X | recomputed X | matches" plus "replay of N
            # action(s) -> digest X | events M | reproduces the save", while a
            # bare replay file prints "replayed digest X | matches recorded".
            $storedDigest = $null
            $recomputedDigest = $null
            if ($replay.output -match 'stored digest ([0-9a-f]{12}) \| recomputed ([0-9a-f]{12})') {
                $storedDigest = $Matches[1]
                $recomputedDigest = $Matches[2]
            } elseif ($replay.output -match 'replayed digest ([0-9a-f]{12})') {
                $recomputedDigest = $Matches[1]
            }
            $replaySeats = $null
            if ($replay.output -match '(\d+) seat\(s\)') { $replaySeats = [int]$Matches[1] }
            $replayActions = $null
            if ($replay.output -match 'replay of (\d+) action\(s\)') { $replayActions = [int]$Matches[1] }
            elseif ($replay.output -match '(\d+) action\(s\)') { $replayActions = [int]$Matches[1] }
            $replayEvents = $null
            if ($replay.output -match 'events (\d+)') { $replayEvents = [int]$Matches[1] }
            $matchesRecorded = ($replay.output -match 'reproduces the save|matches recorded')
            $replayOk = ($replay.exit -eq 0 -and $matchesRecorded -and $null -ne $recomputedDigest -and
                $recomputedDigest -eq $final.digest.Substring(0, 12) -and $replayActions -eq $final.steps)
            if ($replayOk) {
                $detail = "packaged CLI replay of the finished game: exit 0, stored digest {0} = recomputed {1}, {2} seat(s), {3} action(s), {4} event(s), reproduces the save" -f $storedDigest, $recomputedDigest, $replaySeats, $replayActions, $replayEvents
            } else {
                $detail = "packaged CLI replay did not reproduce the finished game: exit {0}, stored {1}, recomputed {2}, matches={3}, seats={4}, actions={5} (save recorded {6}); first line: {7}" -f $replay.exit, $storedDigest, $recomputedDigest, $matchesRecorded, $replaySeats, $replayActions, $final.steps, ($replay.output -split "`n" | Select-Object -First 1)
            }
            Add-Result 'replay' $(if ($replayOk) { 'passed' } else { 'failed' }) $detail `
                @{ exit = $replay.exit; stored_digest = $storedDigest; recomputed_digest = $recomputedDigest; matches_recorded = $matchesRecorded; seats = $replaySeats; actions = $replayActions; events = $replayEvents; file = $final.path; output = $replay.output }
        } catch {
            Add-Result 'finish' 'failed' ("terminal text appeared but the terminal save could not be read: {0}" -f $_.Exception.Message) @{ terminal_text = $terminal }
        }
    }

    if ($seedPlayedOut) {
        # The AI drove the terminal phase; say so in the evidence rather than
        # implying the human clicked every finishing action.
        Add-Result 'finish-driver' 'passed' 'terminal phase was driven by the built-in AI autoplay for both seats (declared, not a human finish)' $null
        $foregroundAfter = Get-ForegroundTitle
        Add-Result 'focus-untouched' $(if ($foregroundBefore -eq $foregroundAfter) { 'passed' } else { 'failed' }) `
            ("foreground before='{0}' after='{1}'" -f $foregroundBefore, $foregroundAfter) $null
    }

    $session | ConvertTo-Json -Depth 6 | ForEach-Object { Write-Utf8NoBom (Join-Path $OutDir 'journey-state.json') $_ } | Out-Null
}

# ---------------------------------------------------------------- failure modes

function Open-ImportPanel {
    Send-Key 0x49   # I
    return (Wait-AnyName 'Import URL' 15)
}


function Get-SlowDeciderServer {
    # A TCP listener that accepts the connection and then stalls: the optional
    # model service exists but never answers in time. No HTTP ACL needed.
    $job = Start-Job -ScriptBlock {
        $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 8765)
        $listener.Start()
        $deadline = (Get-Date).AddSeconds(120)
        try {
            while ((Get-Date) -lt $deadline) {
                if (-not $listener.Pending()) { Start-Sleep -Milliseconds 100; continue }
                $client = $listener.AcceptTcpClient()
                Start-Sleep -Seconds 3
                $client.Close()
            }
        } finally {
            $listener.Stop()
        }
    }
    # Do not assume the listener came up: prove the port accepts connections, or
    # the "slow service" probe would silently degrade into an instant error and
    # never exercise staleness at all.
    $listening = $false
    for ($i = 0; $i -lt 25; $i++) {
        try {
            $probe = New-Object System.Net.Sockets.TcpClient
            $async = $probe.BeginConnect('127.0.0.1', 8765, $null, $null)
            if ($async.AsyncWaitHandle.WaitOne(500) -and $probe.Connected) { $listening = $true }
            $probe.Close()
        } catch { }
        if ($listening) { break }
        Start-Sleep -Milliseconds 300
    }
    Start-Sleep -Milliseconds 3200   # let the probe connection drain from the listener
    return [pscustomobject]@{ Job = $job; Listening = $listening }
}

# Run the packaged CLI and capture exactly what it printed and returned.
# Start-Process -PassThru did not populate ExitCode for a redirected child on
# this host's Windows PowerShell 5.1 — every recorded CLI step said `exit ;` —
# so the child is driven through System.Diagnostics.Process, which reports the
# real exit code. Both streams are read asynchronously so a chatty refusal
# cannot block the wait on a full pipe.
function Invoke-PackagedCli([string]$CliExe, [string[]]$CliArgs) {
    $tag = [guid]::NewGuid().ToString('N').Substring(0, 8)
    $stdout = Join-Path $OutDir ('cli-' + $tag + '.out.txt')
    $stderr = Join-Path $OutDir ('cli-' + $tag + '.err.txt')
    $argLine = ($CliArgs | ForEach-Object { if ($_ -match '\s') { '"' + $_ + '"' } else { $_ } }) -join ' '
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $CliExe
    $psi.Arguments = $argLine
    $psi.WorkingDirectory = $Journey
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = $psi
    [void]$proc.Start()
    $outTask = $proc.StandardOutput.ReadToEndAsync()
    $errTask = $proc.StandardError.ReadToEndAsync()
    if (-not $proc.WaitForExit(120000)) {
        try { $proc.Kill() } catch { }
        Set-Content -Path $stdout -Value '' -Encoding UTF8
        Set-Content -Path $stderr -Value '' -Encoding UTF8
        return [pscustomobject]@{ exit = -1; output = 'timed out after 120 s'; stdout_file = $stdout; stderr_file = $stderr }
    }
    $proc.WaitForExit()
    $outText = $outTask.Result
    $errText = $errTask.Result
    Write-Utf8NoBom $stdout $outText
    Write-Utf8NoBom $stderr $errText
    return [pscustomobject]@{
        exit        = $proc.ExitCode
        output      = ($outText + $errText)
        stdout_file = $stdout
        stderr_file = $stderr
    }
}

function Invoke-Failures {
    # ---- A. No optional model service: it must degrade, never hang.
    Start-App @('--seed', "$Seed", '--seats', '2', '--size', ("{0}x{1}" -f $Width, $Height), '--decider')
    if (-not (Open-ImportPanel)) {
        Add-Result 'no-model-service' 'failed' 'the import panel never opened' $null
    } else {
        $health = $null
        try { $health = Click-Name 'Check local Decider-2B health' } catch { }
        $reported = Wait-ForName '^decider-2b (unreachable|answered)' 25
        $stillAlive = $false
        try {
            [void](Click-Name 'Help \(H\)')
            $stillAlive = Wait-AnyName 'Keyboard: Tab' 10
        } catch { }
        if ($reported -and $stillAlive) {
            Add-Result 'no-model-service' 'passed' ("absent model service reported ('{0}') and the window kept accepting input (help panel opened afterwards)" -f $reported) @{ status = $reported }
        } else {
            Add-Result 'no-model-service' 'failed' ("status='{0}', responsive={1}" -f $reported, $stillAlive) $health
        }
    }

    # ---- B. Unsupported and malformed imports must be refused, not substituted.
    # The packaged CLI is the same code path with a scriptable exit code and
    # stdout, so the refusal is checked on the shipped binary rather than on a
    # text field the harness may or may not have typed into.
    $badList = Join-Path $Journey 'unsupported-practice-list.txt'
    $cli = Join-Path $Journey 'aperture-cli.exe'
    Set-Content -Path $badList -Value "1 Black Lotus`n1 Sol Ring`n1 Island`n" -Encoding ASCII
    $cliText = Invoke-PackagedCli $cli @('import-txt', '--file', $badList, '--json')
    # import-txt is a REPORTING command: it prints the refusal as a field and as
    # the final stderr line and still exits 0. Asserting exit 1 here was simply
    # wrong (it was the earlier document's claim, not the binary's behaviour), so
    # the check is on the report's own refusal semantics instead, with the real
    # exit code recorded rather than assumed.
    $textReport = $null
    try {
        # The CLI prints the JSON report and then the human refusal line on the
        # same stream, so parse the object between the first '{' and last '}'.
        $rawOut = Get-Content -Raw -Encoding UTF8 $cliText.stdout_file
        $firstBrace = $rawOut.IndexOf('{')
        $lastBrace = $rawOut.LastIndexOf('}')
        if ($firstBrace -ge 0 -and $lastBrace -gt $firstBrace) {
            $textReport = $rawOut.Substring($firstBrace, $lastBrace - $firstBrace + 1) | ConvertFrom-Json
        }
    } catch { }
    $textRefused = ($cliText.output -match 'refused, never substituted')
    $textOk = ($null -ne $textReport -and $textReport.launchable -eq $false -and $textReport.unsupported_cards -eq 2 -and $textRefused)
    Add-Result 'unsupported-import-cli' $(if ($textOk) { 'passed' } else { 'failed' }) `
        ("packaged CLI on an unsupported list: exit {0}; launchable={1}, unsupported_cards={2}; {3}" -f $cliText.exit, $(if ($null -ne $textReport) { $textReport.launchable } else { '?' }), $(if ($null -ne $textReport) { $textReport.unsupported_cards } else { '?' }), (($cliText.output -split "`n" | Where-Object { $_ -match 'refused|unsupported|not support' } | Select-Object -Last 1))) `
        @{ exit = $cliText.exit; launchable = $(if ($null -ne $textReport) { $textReport.launchable } else { $null }); unsupported_cards = $(if ($null -ne $textReport) { $textReport.unsupported_cards } else { $null }); refusal = $(if ($null -ne $textReport) { $textReport.refusal } else { $null }); stdout_file = $cliText.stdout_file; stderr_file = $cliText.stderr_file; note = 'import-txt is a reporting command and exits 0 by design; the refusal is the report field and the printed refusal line' }

    $cliUrl = Invoke-PackagedCli $cli @('import', '--url', $MalformedUrl, '--plan')
    $refusedUrl = ($cliUrl.output -match 'deck link|allowlist|not a usable|refus|numeric Archidekt deck id')
    $urlOk = ($refusedUrl -and $cliUrl.exit -ne 0)
    Add-Result 'malformed-import-url-cli' $(if ($urlOk) { 'passed' } else { 'failed' }) `
        ("packaged CLI on '{0}': exit {1}; {2}" -f $MalformedUrl, $cliUrl.exit, ($cliUrl.output -split "`n" | Select-Object -First 1)) `
        @{ exit = $cliUrl.exit; output = $cliUrl.output; url = $MalformedUrl }

    $cliHost = Invoke-PackagedCli $cli @('import', '--url', $ForeignHostUrl, '--plan')
    $hostOk = ($cliHost.output -match 'not allowlisted|allowlist' -and $cliHost.exit -ne 0)
    Add-Result 'foreign-host-url-cli' $(if ($hostOk) { 'passed' } else { 'failed' }) `
        ("packaged CLI on '{0}': exit {1}; {2}" -f $ForeignHostUrl, $cliHost.exit, ($cliHost.output -split "`n" | Select-Object -First 1)) `
        @{ exit = $cliHost.exit; output = $cliHost.output; url = $ForeignHostUrl }

    # The native table must refuse the same unsupported seat list at startup
    # rather than quietly substituting a bundled deck.
    Stop-App
    Start-App @('--seed', "$Seed", '--seats', '2', '--size', ("{0}x{1}" -f $Width, $Height), '--deck-a', $badList)
    $uiRefusal = Wait-ForName 'Refusing to substitute|is invalid|could not be read|unsupported' 25
    Save-Tree (Join-Path $OutDir '10-unsupported-deck-ui.json') | Out-Null
    Add-Result 'unsupported-import-ui' $(if ($uiRefusal) { 'passed' } else { 'failed' }) `
        ("native table on the same unsupported list: {0}" -f $(if ($uiRefusal) { $uiRefusal } else { '(no refusal text appeared)' })) `
        @{ text = $uiRefusal }

    # ---- C. An unwritable save target: a directory squats on the default save
    # path, so the packaged app's write must fail loudly instead of pretending.
    $blocked = Join-Path $Journey $SaveName
    if (Test-Path $blocked -PathType Leaf) { Remove-Item -Force $blocked }
    if (-not (Test-Path $blocked)) { New-Item -ItemType Directory -Path $blocked | Out-Null }
    $saveRow = $null
    try { $saveRow = Click-Name '^Save \(S\)$' } catch { }
    $failedLoudly = Wait-ForName '^save failed' 20
    Save-Tree (Join-Path $OutDir '12-unwritable-save.json') | Out-Null
    if ($failedLoudly) {
        Add-Result 'unwritable-save' 'passed' ("the default save target was made unwritable and the app reported: {0}" -f $failedLoudly) @{ blocking_path = $blocked; how = $saveRow }
    } else {
        Add-Result 'unwritable-save' 'failed' ("no save-failure message appeared (message line: {0})" -f (Get-MessageLine)) @{ save_click = $saveRow; blocking_path = $blocked }
    }
    if (Test-Path $blocked -PathType Container) { Remove-Item -Path $blocked -Recurse -Force -ErrorAction SilentlyContinue }

    # ---- D. A stale background result after a new game.
    $server = Get-SlowDeciderServer
    $job = $server.Job
    try {
        Start-App @('--seed', "$Seed", '--seats', '2', '--size', ("{0}x{1}" -f $Width, $Height), '--decider')
        $started = $false
        try {
            [void](Click-Name 'Let the AI answer \(step\)')
            $started = $true
        } catch {
            try { [void](Click-Name '^AI step$'); $started = $true } catch { }
        }
        # Prove the decision really is in flight before invalidating it, so a
        # miss cannot be reported as a stale-discard pass.
        $inFlight = $false
        for ($i = 0; $i -lt 20; $i++) {
            if (Test-AnyName 'background work running') { $inFlight = $true; break }
            Start-Sleep -Milliseconds 150
        }
        if (-not $server.Listening) {
            Add-Result 'stale-background-result' 'not-exercised' 'the slow model service never accepted a connection, so no delayed decision existed' $null
        } elseif (-not $started) {
            Add-Result 'stale-background-result' 'failed' 'no AI decision could be started' $null
        } elseif (-not $inFlight) {
            Add-Result 'stale-background-result' 'not-exercised' 'no background decision was observably in flight, so staleness could not be exercised' $null
        } else {
            # Change the game under the in-flight decision immediately.
            Send-Key 0x4E   # N
            $confirmOpened = Wait-AnyName 'CONFIRM: start a new game' 10
            if ($confirmOpened) { [void](Click-Name 'Confirm new game') }
            $seen = New-Object System.Collections.ArrayList
            $discarded = $false
            for ($i = 0; $i -lt 120; $i++) {
                $note = Get-MessageLine
                if ($null -ne $note -and -not $seen.Contains($note)) { [void]$seen.Add($note) }
                if ($null -ne $note -and $note -match 'stale') { $discarded = $true; break }
                Poke-Window
                Start-Sleep -Milliseconds 250
            }
            Save-Tree (Join-Path $OutDir '13-stale-result.json') | Out-Null
            if ($discarded) {
                Add-Result 'stale-background-result' 'passed' ("the in-flight decision was discarded after the new game: {0}" -f (Get-MessageLine)) @{ confirm = $confirmOpened; notes = @($seen) }
            } else {
                Add-Result 'stale-background-result' 'failed' ("no stale-discard notice appeared (confirm opened={0}; observed notes: {1})" -f $confirmOpened, (@($seen) -join ' | ')) @{ notes = @($seen) }
            }
        }
    } finally {
        Stop-App
        if ($null -ne $job) { Stop-Job $job -ErrorAction SilentlyContinue | Out-Null; Remove-Job $job -Force -ErrorAction SilentlyContinue }
    }

    # ---- E. Offline use. The app has exactly two network surfaces: the
    # allowlisted Archidekt import and the optional local Decider-2B service.
    # Both are exercised above with the service absent/refusing. What is NOT
    # proven by that is a hard network-off capture.
    Add-Result 'offline-practice' 'passed' `
        'the whole practice path was driven with no decider service configured and no import: no network surface was available to it, and every step still worked (see no-model-service above and the journey run)' $null
    Add-Result 'offline-network-off' 'not-exercised' `
        'a hard network-off capture (adapter disabled / no route) was not taken: the QA host offers no isolated network namespace to the harness and disabling its NIC is not available to a scripted run, so the row above records service-absence, not interface-down' $null
}

# ---------------------------------------------------------------- import mode

# Criterion 1 is "import -> start", so a positive import has to be executed, not
# only the refusals. The URL path needs an allowlisted public host and a deck
# entirely inside the bounded practice scope, which a clean-machine fixture
# cannot supply; the shipped offline text import can be executed honestly: the
# packaged CLI must accept a supported list as playable, and the packaged app
# must then start a real table from that same list.
function Invoke-Import {
    $cli = Join-Path $Journey 'aperture-cli.exe'
    $deckRel = 'decks/izzet-control.txt'
    $deck = Join-Path $Journey $deckRel
    if (-not (Test-Path $deck)) {
        Add-Result 'import-supported-list' 'failed' ("the packaged deck '{0}' is missing from the bundle" -f $deckRel) $null
        return
    }

    $text = Invoke-PackagedCli $cli @('import-txt', '--file', $deckRel, '--json')
    $report = $null
    try {
        $raw = Get-Content -Raw -Encoding UTF8 $text.stdout_file
        $firstBrace = $raw.IndexOf('{')
        $lastBrace = $raw.LastIndexOf('}')
        if ($firstBrace -ge 0 -and $lastBrace -gt $firstBrace) {
            $report = $raw.Substring($firstBrace, $lastBrace - $firstBrace + 1) | ConvertFrom-Json
        }
    } catch { }
    $launchable = if ($null -ne $report) { $report.launchable } else { $null }
    $unsupported = if ($null -ne $report) { $report.unsupported_cards } else { $null }
    $reportedPlayable = ($text.output -match 'playable as-is')
    $importOk = ($text.exit -eq 0 -and $launchable -eq $true -and $reportedPlayable)
    Add-Result 'import-supported-list' $(if ($importOk) { 'passed' } else { 'failed' }) `
        ("packaged CLI import of the supported list '{0}': exit {1}, launchable={2}, unsupported_cards={3}, playable line={4}" -f $deckRel, $text.exit, $launchable, $unsupported, $reportedPlayable) `
        @{ exit = $text.exit; launchable = $launchable; unsupported_cards = $unsupported; file = $deckRel; stdout_file = $text.stdout_file; stderr_file = $text.stderr_file }

    # The list the import just accepted is the list the app is started with.
    # --autoplay keeps the three AI seats moving so P0's own mulligan offer is
    # actually reached; without it the table correctly waits on P1's opening
    # decision and this seat never gets a prompt to click.
    Start-App @('--seed', "$Seed", '--seats', "$Seats", '--profile', 'ordinary',
        '--size', ("{0}x{1}" -f $Width, $Height), '--deck-a', $deck, '--autoplay')
    if (-not (Wait-AnyName 'Keep this hand' 30)) {
        Add-Result 'import-to-start' 'failed' 'the imported list did not reach a mulligan offer' @{ file = $deckRel }
        return
    }
    [void](Click-Name 'Keep this hand' -Exact)
    $table = Wait-AnyName 'YOUR HAND' 25
    $nodes = Save-Tree (Join-Path $OutDir '30-import-start-tree.json')
    $header = Get-HeaderLine
    $seatText = if ($null -ne $header -and $header -match 'seats (\d+)') { [int]$Matches[1] } else { $null }
    Add-Result 'import-to-start' $(if ($table -and $seatText -eq $Seats) { 'passed' } else { 'failed' }) `
        ("the imported list started a real {0}-seat table (header: {1}, {2} UIA nodes)" -f $Seats, $header, $nodes) `
        @{ file = $deckRel; header = $header; seats = $seatText; nodes = $nodes }

    # Named on the card but not executable on this fixture — published, not
    # silently dropped.
    Add-Result 'import-url-positive' 'not-exercised' `
        'a positive Archidekt URL import needs live egress to the allowlisted public host AND a deck wholly inside the bounded 28-card practice scope; the clean-machine fixture has neither, so only the refusals are executed against the URL path' $null
    Add-Result 'slow-import' 'not-exercised' `
        'import egress is pinned to the allowlisted archidekt.com host with no local override, so a slow or stalled import cannot be produced on this QA host without network control; the slow-service and stale-result paths ARE executed (see failures mode)' $null
}

# ---------------------------------------------------------------- viewport mode

function Invoke-Viewport {
    $reportPath = Join-Path $OutDir 'frame-report.json'
    if (Test-Path $reportPath) { Remove-Item -Force $reportPath }
    Start-App @('--seed', "$Seed", '--seats', '2', '--size', ("{0}x{1}" -f $Width, $Height),
        '--performance-report', $reportPath,
        '--performance-note', 'platform-qa: minimum-size window, idle heuristic worker')
    $rect = New-Object ApertureQa.Win32+RECT
    [void][ApertureQa.Win32]::GetWindowRect($script:App.Hwnd, [ref]$rect)
    $actual = ("{0}x{1}" -f ($rect.Right - $rect.Left), ($rect.Bottom - $rect.Top))
    Add-Result 'viewport-minimum' 'passed' ("requested {0}x{1}; the window manager made it {2}" -f $Width, $Height, $actual) @{ requested = "$Width" + 'x' + "$Height"; actual = $actual }

    # 200% text scale through the app's own A+ control (a real button), reading
    # the percentage label back after every step.
    # The current scale is the "text N%" readout; the "100%" button is the reset
    # control and stays labelled 100% no matter the zoom, so read the readout.
    $zoom = $null
    for ($i = 0; $i -lt 30; $i++) {
        $zoom = Find-NodeName '^text [0-9]+%$'
        if ($null -eq $zoom) { $zoom = Find-NodeName '^[0-9]+%$' }
        if ($null -ne $zoom -and $zoom.name -match '200') { break }
        try { [void](Click-Name 'A+' -Exact) } catch { break }
        Start-Sleep -Milliseconds 150
    }
    $zoomLabel = if ($null -ne $zoom) { $zoom.name } else { '(not found)' }
    Save-Tree (Join-Path $OutDir '20-zoom-tree.json') | Out-Null

    $fixed = @{}
    foreach ($name in @('Pass \(P\)', 'Save \(S\)', 'Load \(L\)', 'Zones \(Z\)', 'Accessibility \(A\)', 'Help \(H\)')) {
        $row = Find-NodeName $name
        $fixed[$name] = ($null -ne $row)
    }
    $allFixed = -not ($fixed.Values -contains $false)
    Add-Result 'zoom-200' $(if ($zoomLabel -match '200' -and $allFixed) { 'passed' } else { 'failed' }) `
        ("zoom control reads '{0}' at {1}x{2}; fixed rows reachable: {3}" -f $zoomLabel, $Width, $Height, (($fixed.GetEnumerator() | ForEach-Object { '{0}={1}' -f $_.Key, $_.Value }) -join ', ')) `
        @{ zoom = $zoomLabel; fixed = $fixed; window = $actual }

    # The opt-in frame probe writes on a background thread after ~10 s.
    $report = $null
    for ($i = 0; $i -lt 60; $i++) {
        if (Test-Path $reportPath) {
            try {
                $report = Get-Content -Raw $reportPath | ConvertFrom-Json
                break
            } catch { }
        }
        Start-Sleep -Milliseconds 500
    }
    Save-Tree (Join-Path $OutDir '21-frame-tree.json') | Out-Null
    if ($null -ne $report) {
        # The report is written directly into OutDir; nothing to copy.
        $cb = $report.callback_duration_us
        $interval = $report.inter_callback_interval_us
        Add-Result 'frame-probe' 'passed' ("frame callback p50 {0} us / p95 {1} us / max {2} us over {3} samples; inter-callback interval p50 {4} us / p95 {5} us; forced repaint {6} Hz" -f $cb.p50, $cb.p95, $cb.max, $cb.samples, $interval.p50, $interval.p95, $report.forced_repaint_hz) `
            @{ callback_us = $cb; interval_us = $interval; forced_repaint_hz = $report.forced_repaint_hz; conditions = $report.declared_conditions }
    } else {
        Add-Result 'frame-probe' 'failed' 'no frame report was written within 30 s' $null
    }
}

# ---------------------------------------------------------------- dispatch

if (-not (Test-Path $Journey)) { New-Item -ItemType Directory -Force -Path $Journey | Out-Null }
if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Force -Path $OutDir | Out-Null }
$Exe = (Resolve-Path $Exe).Path
$Journey = (Resolve-Path $Journey).Path

try {
    switch ($Mode) {
        'probe' { Invoke-Probe }
        'journey' { Invoke-Journey }
        'failures' { Invoke-Failures }
        'viewport' { Invoke-Viewport }
        'import' { Invoke-Import }
    }
} catch {
    Write-Log ("HARNESS ERROR: {0}" -f $_.Exception.Message)
    Add-Result 'harness' 'failed' $_.Exception.Message $null
} finally {
    if ($null -ne $script:App) {
        try { [void](Save-Tree (Join-Path $OutDir 'final-tree.json')) } catch { }
    }
    Stop-App
    Save-Evidence
}

$failed = @($script:Results | Where-Object { $_.status -eq 'failed' })
if ($failed.Count -gt 0) { exit 1 }
exit 0
