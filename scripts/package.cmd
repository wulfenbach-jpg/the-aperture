@echo off
rem THE APERTURE -- build the reproducible desktop package.
rem Produces dist\the-aperture-<version>-<commit>\ plus a .zip, a SHA-256
rem manifest and a verification command. See docs/packaging.md.
setlocal
cd /d "%~dp0.."

set "PS=powershell"
where pwsh >nul 2>nul && set "PS=pwsh"

%PS% -NoProfile -ExecutionPolicy Bypass -File "%~dp0package.ps1" %*
set "RC=%ERRORLEVEL%"
endlocal & exit /b %RC%
