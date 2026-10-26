param(
 [Parameter(Mandatory=$true)][long]$WindowHandle,
 [Parameter(Mandatory=$true)][int]$ExpectedPid,
 [ValidateSet('Dump','Key','Invoke','MeasureA','Resize','Scroll')][string]$Action='Dump',
 [string]$Label='', [int]$VirtualKey=65,
 [int]$Iterations=40, [int]$Width=1280,[int]$Height=720,
 [string]$OutputPath=''
)
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class Native22 {
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h,uint msg,IntPtr w,IntPtr l);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h,IntPtr after,int x,int y,int w,int height,uint flags);
 [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h,ref Point p);
 public struct Rect {public int left,top,right,bottom;}
 public struct Point {public int x,y;}
}
'@
$h=[IntPtr]$WindowHandle
[uint32]$actual=0
[void][Native22]::GetWindowThreadProcessId($h,[ref]$actual)
if($actual -ne $ExpectedPid){throw "Wrong process $actual; expected $ExpectedPid, refusing input"}
$beforeForeground=[Native22]::GetForegroundWindow().ToInt64()
function Root { return [System.Windows.Automation.AutomationElement]::FromHandle($h) }
function Elements {
 (Root).FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
}
function Find([string]$name) {
 $condition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty,$name)
 (Root).FindFirst([System.Windows.Automation.TreeScope]::Descendants,$condition)
}
function Key([int]$vk) {
 [void][Native22]::PostMessage($h,0x100,[IntPtr]$vk,[IntPtr]1)
 [void][Native22]::PostMessage($h,0x101,[IntPtr]$vk,[IntPtr]0xC0000001L)
}
$result=@{action=$Action;pid=$ExpectedPid;window=$WindowHandle;timestamp=(Get-Date).ToString('o');dpi=[Native22]::GetDpiForWindow($h)}
switch($Action){
 'Dump' {
   $result.elements=@(Elements | ForEach-Object {@{name=$_.Current.Name;type=$_.Current.ControlType.ProgrammaticName;enabled=$_.Current.IsEnabled;focus=$_.Current.HasKeyboardFocus;offscreen=$_.Current.IsOffscreen;rect=$_.Current.BoundingRectangle.ToString()}})
 }
 'Key' {Key $VirtualKey;Start-Sleep -Milliseconds 100;$result.names=@(Elements | ForEach-Object {$_.Current.Name})}
 'Invoke' {
   $el=Find $Label;if(!$el){throw "Missing: $Label"}
   try {($el.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke()}
   catch {($el.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)).Toggle()}
   Start-Sleep -Milliseconds 150
   $result.names=@(Elements | ForEach-Object {$_.Current.Name})
 }
 'MeasureA' {
   if(!$Label){throw 'Pass -Label with an exact accessibility-panel-only control name'}
   $samples=@();$initial=[bool](Find $Label)
   for($i=0;$i -lt $Iterations;$i++) {
     $expected=-not [bool](Find $Label)
     $watch=[System.Diagnostics.Stopwatch]::StartNew();Key 65
     do { $present=[bool](Find $Label);if($present -eq $expected){break};Start-Sleep -Milliseconds 1 } while($watch.Elapsed.TotalMilliseconds -lt 2000)
     $watch.Stop()
     if($present -ne $expected){throw "A state readback timeout at sample $i"}
     $samples+=@{index=$i;open=$present;milliseconds=$watch.Elapsed.TotalMilliseconds}
     Start-Sleep -Milliseconds 40
   }
   $sorted=@($samples.milliseconds | Sort-Object)
   $result.samples=$samples;$result.p95_ms=$sorted[[math]::Ceiling(0.95*$sorted.Count)-1]
   $result.max_ms=$sorted[-1];$result.count=$sorted.Count
   $result.method='Exact-HWND WM_KEYDOWN/UP A dispatch through first matching UIAutomation panel-state readback; polling overhead included; excludes physical input and GPU presentation.'
 }
 'Resize' {
   # SetWindowPos dimensions include non-client borders; read actual client below.
   [void][Native22]::SetWindowPos($h,[IntPtr]::Zero,0,0,$Width,$Height,0x16)
   Start-Sleep -Milliseconds 200
 }
 'Scroll' {
   $p=New-Object Native22+Point;$p.x=$Width;$p.y=$Height
   $local=($p.y -shl 16) -bor ($p.x -band 0xffff)
   [void][Native22]::PostMessage($h,0x200,[IntPtr]0,[IntPtr]$local)
   [void][Native22]::ClientToScreen($h,[ref]$p)
   $screen=($p.y -shl 16) -bor ($p.x -band 0xffff)
   [void][Native22]::PostMessage($h,0x20A,[IntPtr](-120 -shl 16),[IntPtr]$screen)
   Start-Sleep -Milliseconds 150
 }
}
$r=New-Object Native22+Rect
[void][Native22]::GetClientRect($h,[ref]$r)
$result.client=@{width=$r.right;height=$r.bottom}
$result.foreground_before=$beforeForeground;$result.foreground_after=[Native22]::GetForegroundWindow().ToInt64()
$json=$result | ConvertTo-Json -Depth 8
if($OutputPath){[IO.File]::WriteAllText($OutputPath,$json,[Text.UTF8Encoding]::new($false))}
$json
