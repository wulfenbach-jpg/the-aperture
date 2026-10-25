@echo off
rem THE APERTURE -- verify the packaged practice build.
rem Re-derives every SHA-256 in MANIFEST.sha256, refuses unlisted files, checks
rem BUILD-STAMP.txt against the packaged CLI, then launches aperture.exe in its
rem real --smoke mode. Exit code 0 = verified.
setlocal
cd /d "%~dp0"

set "PS=powershell"
where pwsh >nul 2>nul && set "PS=pwsh"

%PS% -NoProfile -ExecutionPolicy Bypass -File "%~dp0verify-package.ps1" %*
set "RC=%ERRORLEVEL%"
endlocal & exit /b %RC%
