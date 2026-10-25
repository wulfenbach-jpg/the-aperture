param([long]$WindowHandle,[int]$ExpectedPid,[string]$SavePath,[string]$OutputPath)
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$root=[System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$WindowHandle)
if($root.Current.ProcessId -ne $ExpectedPid){throw 'Wrong process; refusing input'}
$events=@()
function Names {
  $r=[System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$WindowHandle)
  $els=$r.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
  return @($els | ForEach-Object {$_.Current.Name})
}
function Click([string]$label){
  $r=[System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$WindowHandle)
  $cond=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty,$label)
  $el=$r.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$cond)
  if(!$el){throw "Missing control: $label"}
  try {$p=$el.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern);$p.Invoke()}
  catch {$p=$el.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern);$p.Toggle()}
  Start-Sleep -Milliseconds 350
  $script:events+=@{action=$label;observed=@(Names | Where-Object {$_ -like 'WAITING*' -or $_ -like 'P0 40*' -or $_ -like '*profile commander-combat*' -or $_ -like '*digest*' -or $_ -like '*holds priority*'})}
}
Click '4 seats'
Click ('bounded 40 life '+[char]0xB7+' Isamaru each seat')
Click 'Start new table with these options'
if(!((Names) -match 'P0 40 / P1 40 / P2 40 / P3 40')){throw 'four-seat40life readback failed'}
Click 'Keep this hand'
Click 'Save (S)'
for($i=0;$i -lt 30 -and !(Test-Path $SavePath);$i++){Start-Sleep -Milliseconds 100}
$save=Get-Content -Raw $SavePath | ConvertFrom-Json
if($save.seats.Count -ne 4 -or $save.game.profile -ne 'CommanderCombat40'){throw 'Saved profile/seat mismatch'}
$digest=$save.digest
Click 'New game'
Click 'Load (L)'
$loaded=@(Names | Where-Object {$_ -like 'loaded *'})
if(!(($loaded -join ' ') -like "*$($digest.Substring(0,12))*")){throw 'Load readback digest mismatch'}
@{pid=$ExpectedPid;window=$WindowHandle;events=$events;saved_digest=$digest;profile=$save.game.profile;seats=$save.seats.Count;pending=$save.game.pending;steps=$save.replay.steps.Count;load_readback=$loaded} | ConvertTo-Json -Depth 8 | Set-Content -Encoding UTF8 $OutputPath
Write-Output "PASS actual UIA setup/keep/save/new/load; seats=$($save.seats.Count) profile=$($save.game.profile) digest=$digest"
