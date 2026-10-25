@echo off
rem THE APERTURE -- packaged practice build launcher.
rem Starts the desktop card table from this folder so the bundled decks\ paths
rem resolve. Pass any aperture.exe flag through, e.g.:
rem   launch.cmd --seats 4 --profile bounded
setlocal
cd /d "%~dp0"

if not exist "aperture.exe" (
  echo.
  echo aperture.exe is missing next to this launcher.
  echo Re-extract the package or run verify-package.cmd for diagnostics.
  echo.
  pause
  exit /b 1
)

start "" "%~dp0aperture.exe" %*
endlocal
