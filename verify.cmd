@echo off
rem Full verification run. Writes evidence\verification.log and evidence JSON.
setlocal
cd /d "%~dp0"
if not exist evidence mkdir evidence
set LOG=evidence\verification-run.log
echo === THE APERTURE verification %DATE% %TIME% === > "%LOG%"

echo --- cargo fmt --check --- >> "%LOG%"
cargo fmt --check >> "%LOG%" 2>&1
echo fmt exit=%errorlevel% >> "%LOG%"

echo --- cargo test --all-targets --- >> "%LOG%"
cargo test --all-targets >> "%LOG%" 2>&1
echo test exit=%errorlevel% >> "%LOG%"

echo --- cargo clippy --all-targets -- -D warnings --- >> "%LOG%"
cargo clippy --all-targets -- -D warnings >> "%LOG%" 2>&1
echo clippy exit=%errorlevel% >> "%LOG%"

echo --- cargo build --release --- >> "%LOG%"
cargo build --release >> "%LOG%" 2>&1
echo build exit=%errorlevel% >> "%LOG%"

echo --- selfplay seeds 1..5 --- >> "%LOG%"
for %%S in (1 2 3 4 5) do (
  target\release\aperture-cli.exe selfplay --seed %%S --out evidence\selfplay-seed%%S.json --replay-out evidence\replay-seed%%S.json >> "%LOG%" 2>&1
)

echo --- replay verification --- >> "%LOG%"
target\release\aperture-cli.exe replay --file evidence\replay-seed1.json >> "%LOG%" 2>&1

echo --- live Archidekt import --- >> "%LOG%"
target\release\aperture-cli.exe import --url https://archidekt.com/decks/26206384 --cache cache --out evidence\import-live.json --plan >> "%LOG%" 2>&1

echo --- native UI smoke screenshot --- >> "%LOG%"
if not exist evidence\screenshots mkdir evidence\screenshots
target\release\aperture.exe --smoke evidence\screenshots\ui.png >> "%LOG%" 2>&1

echo Verification finished. Log: %LOG%
echo (the committed run from development is evidence\verification.log)
endlocal
