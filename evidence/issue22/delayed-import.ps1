param([string]$PipeName='aperture-issue22-delay',[int]$DelayMs=12000,[string]$OutputPath='')
# Synthetic slow file/network-like import; never opens a remote endpoint.
# Start this script first, then import \\.\pipe\aperture-issue22-delay in P0's file fallback.
$server=[IO.Pipes.NamedPipeServerStream]::new($PipeName,[IO.Pipes.PipeDirection]::Out)
try {
 Write-Output "READY \\.\pipe\$PipeName"
 $server.WaitForConnection()
 $started=Get-Date
 Write-Output 'CONNECTED; synthetic delay active'
 Start-Sleep -Milliseconds $DelayMs
 $bytes=[Text.Encoding]::UTF8.GetBytes("20 Forest`n20 Grizzly Bears`n")
 $server.Write($bytes,0,$bytes.Length);$server.Flush()
 $result=@{condition='Synthetic delayed named-pipe file import, not real network latency';pipe=$PipeName;delay_ms=$DelayMs;connected_at=$started.ToString('o');completed_at=(Get-Date).ToString('o');bytes=$bytes.Length}
 if($OutputPath){[IO.File]::WriteAllText($OutputPath,($result | ConvertTo-Json),[Text.UTF8Encoding]::new($false))}
 $result | ConvertTo-Json
} finally {$server.Dispose()}
