@echo off
rem Full verification run. Writes evidence\verification-run.log and evidence JSON.
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

echo --- selfplay seed 101 (supervisor gate) --- >> "%LOG%"
target\release\aperture-cli.exe selfplay --seed 101 --out evidence\fix-seed101.json --replay-out evidence\fix-replay101.json >> "%LOG%" 2>&1
echo selfplay-101 exit=%errorlevel% >> "%LOG%"

echo --- replay verification --- >> "%LOG%"
target\release\aperture-cli.exe replay --file evidence\replay-seed1.json >> "%LOG%" 2>&1
target\release\aperture-cli.exe replay --file evidence\fix-replay101.json >> "%LOG%" 2>&1

echo --- swapped-deck seeds 11,22,...,88 (both seats must be able to win) --- >> "%LOG%"
for %%S in (11 22 33 44 55 66 77 88) do (
  target\release\aperture-cli.exe selfplay --seed %%S --deck-a decks\izzet-control.txt --deck-b decks\temur-beats.txt --out evidence\fix-swapped-seed%%S.json --replay-out evidence\fix-swapped-replay%%S.json >> "%LOG%" 2>&1
)

echo --- live Archidekt import (expect an honest refusal) --- >> "%LOG%"
target\release\aperture-cli.exe import --url https://archidekt.com/decks/26206384 --cache cache --out evidence\fix-import-live.json --plan >> "%LOG%" 2>&1
echo import exit=%errorlevel% >> "%LOG%"

echo --- local Decider-2B health + real integration probe (usage checked) --- >> "%LOG%"
target\release\aperture-cli.exe decider-health >> "%LOG%" 2>&1
target\release\aperture-cli.exe selfplay --seed 21 --decider-seat 0 --max-steps 120 --out evidence\fix-decider-probe.json --replay-out evidence\fix-decider-replay.json >> "%LOG%" 2>&1
echo decider-probe exit=%errorlevel% >> "%LOG%"

echo --- native UI smoke screenshots --- >> "%LOG%"
if not exist evidence\screenshots mkdir evidence\screenshots
target\release\aperture.exe --smoke evidence\screenshots\ui-supervisor-fixes.png >> "%LOG%" 2>&1
target\release\aperture.exe --smoke evidence\screenshots\ui-import-refusal.png --deck-a decks\does-not-exist.txt >> "%LOG%" 2>&1

echo Verification finished. Log: %LOG%
echo (the committed run from development is evidence\verification.log)
endlocal
