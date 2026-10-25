<#
    THE APERTURE -- reproducible desktop package builder.

    One command produces a version/commit-stamped practice bundle with a
    SHA-256 manifest and a verifier that re-derives it:

        windows:  scripts\package.cmd
        anywhere: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package.ps1

    Output (dist/ is git-ignored):
        dist/the-aperture-<version>-<shortsha>/        unpacked bundle
        dist/the-aperture-<version>-<shortsha>.zip     same bundle, fixed timestamps

    Bundle contents: aperture.exe, aperture-cli.exe, decks/*.txt,
    BUILD-STAMP.txt, README-PACKAGE.md, launch.cmd, verify-package.cmd,
    verify-package.ps1 and MANIFEST.sha256.

    The build is refused on a dirty working tree (the stamp would otherwise
    name a commit that does not contain the shipped sources); pass -AllowDirty
    for a local experiment, which marks the package "-dirty".

    See docs/packaging.md for the full contract and the honest reproducibility
    statement.
#>
[CmdletBinding()]
param(
    [string]$Version = '',
    [string]$OutDir = 'dist',
    [switch]$AllowDirty,
    [switch]$SkipBuild,
    [switch]$SkipVerify,
    [switch]$SkipSmoke,
    [switch]$NoZip
)

$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$utf8 = New-Object System.Text.UTF8Encoding($false)

function Write-Step([string]$Message) {
    Write-Host ''
    Write-Host ('== ' + $Message) -ForegroundColor Cyan
}

function Invoke-Native([string]$Exe, [string[]]$Arguments) {
    # cargo writes progress to stderr; under $ErrorActionPreference='Stop' PS 5.1
    # would turn those redirected lines into terminating errors.
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = & $Exe @Arguments 2>&1
        $code = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previous
    }
    if ($code -ne 0) {
        throw ($Exe + ' ' + ($Arguments -join ' ') + ' failed (exit ' + $code + '): ' + (($output | Out-String).Trim()))
    }
    return ((($output | Out-String).Trim()) -replace "`r`n", "`n")
}

function Write-TextFile([string]$Path, [string]$Text, [string]$Newline) {
    $normalised = $Text -replace "`r`n", "`n" -replace "`r", "`n"
    if ($Newline -eq "`r`n") { $normalised = $normalised -replace "`n", "`r`n" }
    [System.IO.File]::WriteAllText($Path, $normalised, $utf8)
}

function Copy-NormalisedText([string]$Source, [string]$Target, [string]$Newline) {
    Write-TextFile -Path $Target -Text ([System.IO.File]::ReadAllText($Source)) -Newline $Newline
}

function Get-Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

Push-Location $repo
try {
    Write-Step 'Reading version, commit and toolchain'

    if (-not $Version) {
        $cargoToml = [System.IO.File]::ReadAllText((Join-Path $repo 'Cargo.toml'))
        $versionMatch = [regex]::Match($cargoToml, '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"')
        if (-not $versionMatch.Success) { throw 'cannot read [workspace.package] version from Cargo.toml' }
        $Version = $versionMatch.Groups[1].Value
    }

    $commit = Invoke-Native 'git' @('rev-parse', 'HEAD')
    if ($commit -notmatch '^[0-9a-f]{40}$') { throw ('git rev-parse HEAD returned an unexpected value: ' + $commit) }
    $short = $commit.Substring(0, 7)
    $commitDate = Invoke-Native 'git' @('show', '-s', '--format=%cI', $commit)

    $dirty = ((Invoke-Native 'git' @('status', '--porcelain'))).Length -gt 0
    if ($dirty -and -not $AllowDirty) {
        throw 'refusing to package a dirty working tree (the version/commit stamp would not describe the shipped sources). Commit your work, or pass -AllowDirty for a clearly marked "-dirty" local package.'
    }

    $rustc = Invoke-Native 'rustc' @('--version')
    $cargoVersion = Invoke-Native 'cargo' @('--version')
    $hostInfo = Invoke-Native 'rustc' @('-vV')
    $targetMatch = [regex]::Match($hostInfo, '(?m)^host:\s*(\S+)$')
    if (-not $targetMatch.Success) { throw 'cannot determine the rustc host target triple' }
    $target = $targetMatch.Groups[1].Value
    $rustflags = ''
    if ($env:RUSTFLAGS) { $rustflags = $env:RUSTFLAGS }

    $suffix = $Version + '-' + $short
    if ($dirty) { $suffix = $suffix + '-dirty' }
    $name = 'the-aperture-' + $suffix

    if (-not [System.IO.Path]::IsPathRooted($OutDir)) { $OutDir = Join-Path $repo $OutDir }
    $packageDir = Join-Path $OutDir $name

    Write-Host ('   name    ' + $name)
    Write-Host ('   commit  ' + $commit + ' (' + $commitDate + ')')
    Write-Host ('   target  ' + $target)
    Write-Host ('   rustc   ' + $rustc)

    Write-Step 'Building the release executables (cargo build --release --locked)'
    if (-not $SkipBuild) {
        Invoke-Native 'cargo' @('build', '--release', '--locked', '-p', 'aperture-ui', '-p', 'aperture-cli') | Out-Null
    }
    else {
        Write-Host '   skipped (-SkipBuild)'
    }

    $uiExe = Join-Path $repo 'target/release/aperture.exe'
    $cliExe = Join-Path $repo 'target/release/aperture-cli.exe'
    foreach ($exe in @($uiExe, $cliExe)) {
        if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw ('missing build output: ' + $exe) }
    }

    Write-Step ('Assembling ' + $packageDir)
    if (Test-Path -LiteralPath $packageDir) { Remove-Item -LiteralPath $packageDir -Recurse -Force }
    New-Item -ItemType Directory -Force -Path (Join-Path $packageDir 'decks') | Out-Null

    Copy-Item -LiteralPath $uiExe -Destination (Join-Path $packageDir 'aperture.exe')
    Copy-Item -LiteralPath $cliExe -Destination (Join-Path $packageDir 'aperture-cli.exe')

    $deckSource = Join-Path $repo 'decks'
    $deckFiles = Get-ChildItem -LiteralPath $deckSource -Filter '*.txt' -File | Sort-Object Name
    if ($deckFiles.Count -eq 0) { throw 'no bundled deck lists found in decks/*.txt' }
    foreach ($deck in $deckFiles) {
        Copy-NormalisedText -Source $deck.FullName -Target (Join-Path $packageDir ('decks/' + $deck.Name)) -Newline "`n"
    }

    Copy-NormalisedText -Source (Join-Path $repo 'packaging/README-PACKAGE.md') -Target (Join-Path $packageDir 'README-PACKAGE.md') -Newline "`n"
    Copy-NormalisedText -Source (Join-Path $repo 'packaging/launch.cmd') -Target (Join-Path $packageDir 'launch.cmd') -Newline "`r`n"
    Copy-NormalisedText -Source (Join-Path $repo 'packaging/verify-package.cmd') -Target (Join-Path $packageDir 'verify-package.cmd') -Newline "`r`n"
    Copy-NormalisedText -Source (Join-Path $repo 'packaging/verify-package.ps1') -Target (Join-Path $packageDir 'verify-package.ps1') -Newline "`r`n"

    $stampLines = @(
        'format=1'
        'name=the-aperture'
        ('version=' + $Version)
        ('commit=' + $commit)
        ('commit_short=' + $short)
        ('commit_date=' + $commitDate)
        ('source_dirty=' + $dirty.ToString().ToLowerInvariant())
        'profile=release'
        ('target=' + $target)
        ('rustc=' + $rustc)
        ('cargo=' + $cargoVersion)
        ('rustflags=' + $rustflags)
        'scope=bounded-28-card-practice'
        'signed=false'
    )
    Write-TextFile -Path (Join-Path $packageDir 'BUILD-STAMP.txt') -Text (($stampLines -join "`n") + "`n") -Newline "`n"

    Write-Step 'Writing MANIFEST.sha256 (sorted, LF)'
    $relative = New-Object 'System.Collections.Generic.List[string]'
    foreach ($file in (Get-ChildItem -LiteralPath $packageDir -Recurse -File -Force)) {
        $rel = $file.FullName.Substring($packageDir.Length) -replace '^[\\/]+', ''
        [void]$relative.Add($rel.Replace('\', '/'))
    }
    $sorted = $relative.ToArray()
    [Array]::Sort($sorted, [System.StringComparer]::Ordinal)
    $manifest = New-Object System.Text.StringBuilder
    foreach ($rel in $sorted) {
        $full = Join-Path $packageDir ($rel.Replace('/', '\'))
        [void]$manifest.Append((Get-Sha256 $full) + '  ' + $rel + "`n")
    }
    Write-TextFile -Path (Join-Path $packageDir 'MANIFEST.sha256') -Text $manifest.ToString() -Newline "`n"
    Write-Host ('   ' + $sorted.Count + ' files hashed')

    $zipPath = ''
    if (-not $NoZip) {
        Write-Step 'Writing the distributable zip (fixed entry timestamps)'
        Add-Type -AssemblyName System.IO.Compression | Out-Null
        Add-Type -AssemblyName System.IO.Compression.FileSystem | Out-Null
        $zipPath = Join-Path $OutDir ($name + '.zip')
        if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath -Force }
        $zipEntries = New-Object 'System.Collections.Generic.List[string]'
        foreach ($rel in $sorted) { [void]$zipEntries.Add($rel) }
        [void]$zipEntries.Add('MANIFEST.sha256')
        $zipSorted = $zipEntries.ToArray()
        [Array]::Sort($zipSorted, [System.StringComparer]::Ordinal)
        $fixedTime = New-Object System.DateTimeOffset(2000, 1, 1, 0, 0, 0, [System.TimeSpan]::Zero)
        $archive = [System.IO.Compression.ZipFile]::Open($zipPath, [System.IO.Compression.ZipArchiveMode]::Create)
        try {
            foreach ($rel in $zipSorted) {
                $entry = $archive.CreateEntry($name + '/' + $rel, [System.IO.Compression.CompressionLevel]::Optimal)
                $entry.LastWriteTime = $fixedTime
                $source = Join-Path $packageDir ($rel.Replace('/', '\'))
                $entryStream = $entry.Open()
                try {
                    $fileStream = [System.IO.File]::OpenRead($source)
                    try { $fileStream.CopyTo($entryStream) } finally { $fileStream.Dispose() }
                }
                finally { $entryStream.Dispose() }
            }
        }
        finally { $archive.Dispose() }
        Write-Host ('   ' + $zipPath)
    }

    if (-not $SkipVerify) {
        Write-Step 'Verifying the produced package (hashes, stamp, packaged launch)'
        $verifyScript = Join-Path $packageDir 'verify-package.ps1'
        $verifyArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $verifyScript)
        if ($SkipSmoke) { $verifyArgs += '-SkipSmoke' }
        & powershell @verifyArgs
        if ($LASTEXITCODE -ne 0) { throw ('package verification failed (exit ' + $LASTEXITCODE + ')') }
    }
    else {
        Write-Host ''
        Write-Host 'skip  package verification (-SkipVerify)'
    }

    Write-Step 'Package summary'
    Write-Host ('   package   ' + $packageDir)
    Write-Host ('   manifest  ' + (Join-Path $packageDir 'MANIFEST.sha256'))
    Write-Host ('   stamp     ' + (Join-Path $packageDir 'BUILD-STAMP.txt'))
    Write-Host ('   files     ' + $sorted.Count)
    Write-Host ('   ui        ' + (Get-Sha256 (Join-Path $packageDir 'aperture.exe')))
    Write-Host ('   cli       ' + (Get-Sha256 (Join-Path $packageDir 'aperture-cli.exe')))
    if ($zipPath) { Write-Host ('   zip       ' + $zipPath + '  sha256 ' + (Get-Sha256 $zipPath)) }
    Write-Host ''
    Write-Host ('Verify any copy with: ' + (Join-Path $packageDir 'verify-package.cmd')) -ForegroundColor Green
    Write-Host 'Rebuild the same commit to re-derive the same contents; see docs/packaging.md for what is and is not claimed.'
}
finally {
    Pop-Location
}
