# THE APERTURE — practice build package

**This is a practice slice, not a Magic: The Gathering implementation.** It is an
unsigned, self-contained Windows desktop build of the same bounded practice
program documented in the repository README.

## Read this first — honest scope

* **Unsigned binary.** Windows SmartScreen may warn. There is no code-signing
  certificate, no installer, no auto-updater and no telemetry. The package is a
  folder you can inspect; nothing runs until you start it.
* **Bounded 28-card practice pool.** The engine implements a fixed, explicitly
  enumerated set of 28 real cards (listed in the repository README) with real
  priority/stack/combat scheduling. Cards outside that pool are refused, never
  guessed.
* **Named practice profiles, NOT Commander.** `ordinary` is 20 life, 2–4 seats.
  `bounded` is 40 life with one supported Isamaru, Hound of Konda per seat and a
  bounded combat-damage ledger — a practice profile that exercises commander
  concepts, **not** the Commander format (no 100-card singleton enforcement in
  play, no partners, no legend rule, no full APNAP hidden-zone choice matrix).
* **No network requirement.** The desktop table and the offline CLI run fully
  offline. The optional `import` subcommand is the only networked feature and it
  is allowlisted to public Archidekt deck URLs.

## Contents

| File | What it is |
| --- | --- |
| `aperture.exe` | Native desktop card table (eframe/egui). |
| `aperture-cli.exe` | Headless CLI: `selfplay`, `replay`, `harness`, `import`, `pool`, `decks`, `coach`. |
| `decks/*.txt` | The bundled practice deck lists the desktop table loads by default. |
| `BUILD-STAMP.txt` | Version, exact git commit, target triple and toolchain of this build. |
| `MANIFEST.sha256` | SHA-256 of every payload file above, in this layout. |
| `verify-package.ps1` / `verify-package.cmd` | One command that re-derives and checks every hash, then launches the packaged exe in its smoke mode. |
| `launch.cmd` | Double-click launcher that starts the desktop table from this folder. |
| `README-PACKAGE.md` | This file. |

## Run it

Double-click `launch.cmd`, or run `aperture.exe` from this folder (the bundled
`decks/` paths are resolved relative to the working directory, so start it from
here). Optional flags:

```
aperture.exe --seats 4 --profile bounded      # 4-seat, 40-life bounded practice
aperture.exe --seats 2 --deck-a decks/temur-beats.txt --deck-b decks/izzet-control.txt
aperture.exe --smoke out.png                  # render, screenshot the real window, exit
```

Headless practice and replay verification:

```
aperture-cli.exe selfplay --seats 4 --seed 7 --out game.json --replay-out replay.json
aperture-cli.exe replay --file replay.json
```

## Verify this package

```
verify-package.cmd
```

Exit code `0` means: every file in `MANIFEST.sha256` re-hashes exactly, no
unlisted file was added, `BUILD-STAMP.txt` is consistent with the packaged CLI,
and `aperture.exe --smoke` launched the real window, wrote a PNG and exited
cleanly. Any mismatch exits non-zero and names the offending path.

To check by hand with coreutils, from this folder:

```
sha256sum -c MANIFEST.sha256
```

## Reproducibility — what is and is not claimed

* **Contents and hashes are reproducible.** The bundle is assembled entirely from
  files in the repository at the stamped commit: the two release executables,
  `decks/*.txt`, the packaged README/launcher/verifier templates and
  `BUILD-STAMP.txt`. Text payloads are normalised to LF (`.cmd`/`.ps1` to CRLF)
  while packaging, so the bundle does not depend on a checkout's line-ending
  settings. `MANIFEST.sha256` is sorted ordinal-by-path and written with LF, so
  the same inputs produce the same manifest bytes.
* **Binaries:** the release profile links with MSVC `/Brepro`, so repeated
  builds of the same commit with the same toolchain on the same host produce
  byte-identical executables. Builds on other hosts, toolchain versions or
  operating systems are **not** claimed to be bit-identical; the manifest binds
  the exact artifacts in *this* folder, and `BUILD-STAMP.txt` records the
  toolchain that produced them.
* The `MANIFEST.sha256` in this folder is a tamper-evidence record of these
  bytes. Re-run `scripts/package.ps1` from the repository to rebuild a package
  for the same commit and compare manifests.

## Licence

Original source: MIT OR Apache-2.0 (see the repository `LICENSE-MIT` and
`LICENSE-APACHE`). The repository's `ASSETS.md` records the separate
third-party content boundaries; this package ships no card images or Wizards
assets.
