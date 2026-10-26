param([long]$Window,[string]$Script,[string]$Out)
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$root=[System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$Window)
$hwnd=$Window
$result = @(& ([scriptblock]::Create($Script)))
$result | ConvertTo-Json -Depth 6 | Set-Content -Encoding UTF8 $Out
