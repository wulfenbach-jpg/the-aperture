param([long]$WindowHandle,[string]$Out)
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$h=[IntPtr]$WindowHandle
function Root { [System.Windows.Automation.AutomationElement]::FromHandle($h) }
function Names { @((Root).FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition) | ForEach-Object {$_.Current.Name}) }
function Check([string]$label) {
  $c=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty,$label)
  $el=(Root).FindFirst([System.Windows.Automation.TreeScope]::Descendants,$c)
  if(!$el){throw "missing control: $label"}
  ($el.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)).Toggle(); Start-Sleep -Milliseconds 300
}
$log=[ordered]@{}
$log.hc_before=@(Names | Where-Object {$_ -like 'High contrast:*'})
$log.rm_before=@(Names | Where-Object {$_ -like 'Reduced motion:*'})
Check 'Reduced motion (static busy text, no animation)'
$log.rm_after=@(Names | Where-Object {$_ -like 'Reduced motion:*'})
Check 'High contrast (white text, near-black surfaces, bright borders)'
$log.hc_after=@(Names | Where-Object {$_ -like 'High contrast:*'})
$log.zoom_control=@(Names | Where-Object {$_ -eq '100%' -or $_ -like 'text *%'})
$log.fixed_actions=@(Names | Where-Object {$_ -eq 'YOUR ACTIONS — only engine-offered decisions are accepted'})
$log.status_words=@(Names | Where-Object {$_ -match 'WAITING|holds priority|CERTIFIED|REFUSED|Working'})[0..2]
$log.seat_panels=@(Names | Where-Object {$_ -match '^P[0-3] · (YOU|AI|Command)' -or $_ -match '^P[0-3] '})[0..5]
$r=[System.Windows.Automation.AutomationElement]::FromHandle($h).Current.BoundingRectangle
$log.window_rect="$($r.Width)x$($r.Height)"
$log | ConvertTo-Json -Depth 6 | Set-Content -Encoding UTF8 $Out
$log | ConvertTo-Json -Depth 6
