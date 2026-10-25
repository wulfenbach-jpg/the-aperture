<#
    THE APERTURE -- packaged-bundle verifier (ships inside the package).

    Re-derives the SHA-256 of every file listed in MANIFEST.sha256, refuses any
    unlisted payload file, checks BUILD-STAMP.txt against the packaged CLI, and
    unless -SkipSmoke launches aperture.exe in its real --smoke mode and checks
    the captured PNG. Exit code 0 means the package is intact and launches.

    Usage:
        powershell -NoProfile -ExecutionPolicy Bypass -File verify-package.ps1
        powershell -NoProfile -ExecutionPolicy Bypass -File verify-package.ps1 -SkipSmoke
#>
[CmdletBinding()]
param(
    [switch]$SkipSmoke,
    [string]$Manifest = 'MANIFEST.sha256'
)

$ErrorActionPreference = 'Stop'
$script:failures = 0
$script:checked = 0

function Write-Fail([string]$Message) {
    Write-Host ('FAIL  ' + $Message) -ForegroundColor Red
    $script:failures++
}

function Write-Ok([string]$Message) {
    Write-Host ('ok    ' + $Message) -ForegroundColor Green
}

Push-Location $PSScriptRoot
try {
    $root = (Get-Location).Path

    if (-not (Test-Path -LiteralPath $Manifest -PathType Leaf)) {
        Write-Fail ("manifest '" + $Manifest + "' is missing")
        exit 1
    }
    $manifestRel = $Manifest.Replace('\', '/')

    # ---- 1. re-derive and compare every listed hash ------------------------
    $expected = @{}
    $order = New-Object 'System.Collections.Generic.List[string]'
    foreach ($line in [System.IO.File]::ReadAllLines((Join-Path $root $Manifest))) {
        if ($line.Trim().Length -eq 0) { continue }
        if ($line -notmatch '^([0-9a-fA-F]{64}) [ *](.+)$') {
            Write-Fail ('malformed manifest line: ' + $line)
            continue
        }
        $hash = $Matches[1].ToLowerInvariant()
        $rel = $Matches[2]
        if ($expected.ContainsKey($rel)) {
            Write-Fail ('duplicate manifest entry: ' + $rel)
            continue
        }
        $expected[$rel] = $hash
        [void]$order.Add($rel)
    }
    if ($expected.Count -eq 0) {
        Write-Fail 'manifest lists no files'
    }

    foreach ($rel in $order) {
        $full = Join-Path $root ($rel.Replace('/', '\'))
        if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
            Write-Fail ('missing file: ' + $rel)
            continue
        }
        $actual = (Get-FileHash -LiteralPath $full -Algorithm SHA256).Hash.ToLowerInvariant()
        $script:checked++
        if ($actual -ne $expected[$rel]) {
            Write-Fail ('hash mismatch: ' + $rel + ' (expected ' + $expected[$rel] + ', got ' + $actual + ')')
        }
        else {
            Write-Ok ($rel + '  ' + $actual)
        }
    }

    # ---- 2. no unlisted payload files -------------------------------------
    $listed = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::Ordinal)
    foreach ($rel in $order) { [void]$listed.Add($rel) }
    foreach ($file in (Get-ChildItem -LiteralPath $root -Recurse -File -Force)) {
        $rel = $file.FullName.Substring($root.Length) -replace '^[\\/]+', ''
        $rel = $rel.Replace('\', '/')
        if ($rel -eq $manifestRel) { continue }
        if ($rel -like 'smoke-*.png') { continue }
        if (-not $listed.Contains($rel)) {
            Write-Fail ('unlisted file in bundle: ' + $rel)
        }
    }

    # ---- 3. version / commit stamp ----------------------------------------
    $stampPath = Join-Path $root 'BUILD-STAMP.txt'
    if (-not (Test-Path -LiteralPath $stampPath -PathType Leaf)) {
        Write-Fail 'BUILD-STAMP.txt is missing'
    }
    else {
        $stamp = @{}
        foreach ($line in [System.IO.File]::ReadAllLines($stampPath)) {
            $at = $line.IndexOf('=')
            if ($at -gt 0) { $stamp[$line.Substring(0, $at)] = $line.Substring($at + 1) }
        }
        foreach ($key in @('format', 'version', 'commit', 'target', 'rustc', 'scope')) {
            if (-not $stamp.ContainsKey($key) -or $stamp[$key].Trim().Length -eq 0) {
                Write-Fail ("BUILD-STAMP.txt is missing '" + $key + "'")
            }
        }
        if ($stamp.ContainsKey('commit') -and $stamp['commit'] -notmatch '^[0-9a-f]{40}$') {
            Write-Fail ('BUILD-STAMP.txt commit is not a full 40-hex git SHA: ' + $stamp['commit'])
        }
        if ($stamp.ContainsKey('signed') -and $stamp['signed'] -ne 'false') {
            Write-Fail ('BUILD-STAMP.txt must record signed=false for this unsigned package')
        }

        $cli = Join-Path $root 'aperture-cli.exe'
        if (-not (Test-Path -LiteralPath $cli -PathType Leaf)) {
            Write-Fail 'aperture-cli.exe is missing from the bundle'
        }
        else {
            $cliVersion = ((& $cli --version) -join ' ').Trim()
            if ($stamp.ContainsKey('version')) {
                if ($cliVersion -notmatch [regex]::Escape($stamp['version'])) {
                    Write-Fail ("aperture-cli --version ('" + $cliVersion + "') does not report the stamped version '" + $stamp['version'] + "'")
                }
                else {
                    Write-Ok ('stamped version ' + $stamp['version'] + ' confirmed by aperture-cli --version')
                }
            }
        }
        Write-Ok ('commit ' + $stamp['commit'] + ' target ' + $stamp['target'] + ' rustc ' + $stamp['rustc'])
    }

    # ---- 4. the packaged desktop executable actually launches -------------
    if (-not $SkipSmoke) {
        $exe = Join-Path $root 'aperture.exe'
        if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
            Write-Fail 'aperture.exe is missing from the bundle'
        }
        else {
            $shot = Join-Path ([System.IO.Path]::GetTempPath()) ('aperture-smoke-' + [guid]::NewGuid().ToString('N') + '.png')
            $smokeOut = (& $exe --smoke $shot 2>&1 | Out-String)
            $code = $LASTEXITCODE
            if ($code -ne 0) {
                Write-Fail ('aperture.exe --smoke exited ' + $code + ': ' + $smokeOut.Trim())
            }
            elseif (-not (Test-Path -LiteralPath $shot -PathType Leaf)) {
                Write-Fail ('aperture.exe --smoke printed "' + $smokeOut.Trim() + '" but wrote no screenshot')
            }
            else {
                $bytes = [System.IO.File]::ReadAllBytes($shot)
                if ($bytes.Length -lt 1024) {
                    Write-Fail ('smoke screenshot is suspiciously small (' + $bytes.Length + ' bytes)')
                }
                elseif ($bytes[0] -ne 0x89 -or $bytes[1] -ne 0x50 -or $bytes[2] -ne 0x4E -or $bytes[3] -ne 0x47) {
                    Write-Fail 'smoke screenshot is not a PNG'
                }
                else {
                    Write-Ok ('aperture.exe --smoke launched the packaged desktop build (' + $bytes.Length + ' byte PNG)')
                }
                Remove-Item -LiteralPath $shot -Force -ErrorAction SilentlyContinue
            }
        }
    }
    else {
        Write-Host 'skip  packaged desktop launch smoke (-SkipSmoke)'
    }

    # ---- summary ----------------------------------------------------------
    if ($script:failures -gt 0) {
        Write-Host ''
        Write-Host ('PACKAGE VERIFICATION FAILED: ' + $script:failures + ' problem(s); ' + $script:checked + ' file hashes re-derived') -ForegroundColor Red
        exit 1
    }
    Write-Host ''
    Write-Host ('PACKAGE VERIFIED: ' + $script:checked + ' file hashes re-derived, contents match MANIFEST.sha256') -ForegroundColor Green
    exit 0
}
finally {
    Pop-Location
}
