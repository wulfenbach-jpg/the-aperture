@echo off
rem THE APERTURE — double-click launcher (practice slice; supported pool; NOT Commander)
setlocal
cd /d "%~dp0"

where cargo >nul 2>nul
if errorlevel 1 (
  echo.
  echo Rust is not on PATH. Install the MSVC Rust toolchain from https://rustup.rs and try again.
  echo.
  pause
  exit /b 1
)

echo Building THE APERTURE (first run downloads and compiles its dependencies)...
cargo build --release -p aperture-ui
if errorlevel 1 (
  echo.
  echo Build failed. See the output above.
  pause
  exit /b 1
)

echo Starting the card table...
start "" "%~dp0target\release\aperture.exe" %*
if errorlevel 1 (
  echo The window could not be started.
  pause
  exit /b 1
)
endlocal
