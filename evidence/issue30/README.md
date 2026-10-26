# Evidence — #30 platform and accessibility acceptance

Machine-recorded output from `scripts/platform_qa/platform-qa.ps1` runs against the
packaged Windows build `the-aperture-0.1.0-51fbc6b`. Nothing here is hand-written
except this index; the numbers in `docs/platform-acceptance.md` come from these
files.

## package/

* `BUILD-STAMP.txt` — the stamp inside the QA target: `commit=51fbc6b…`,
  `source_dirty=false`, rustc 1.97.1, `x86_64-pc-windows-msvc`, `signed=false`.
* `MANIFEST.sha256` — the 12-file manifest the packaged verifier re-derived before
  QA started (it also rendered a smoke PNG from the packaged binary).

## journey/

The scripted native-input journey (launch → hand → priority → stack → save → load →
autoplay → finish).

* `harness-run.json` — every step with its status, the observed text, and the full
  transcript; includes the SHA-256 of the exact `aperture.exe` that was driven.
* `01-launch-tree.json`, `03-table-tree.json`, `04-stack-tree.json`,
  `06-loaded-tree.json` — the live accessibility trees at the points where the
  journey changed state (the hand card offers, the stack sitting at 1, the loaded
  position).
* `the-aperture-save.json` — the file the packaged app wrote, captured as evidence
  of the save size, version, digest and replay length.

## failures/

* `harness-run.json` — no-model-service, unsupported-import (CLI + native), and
  unwritable-save probes.
* `harness-run-stale-passed.json` — the run where the stale-background-result probe
  actually captured `discarded a stale AI decision [stale_generation]`. Later runs
  did not reproduce it; the document says so rather than citing only the pass.

## viewport/

* `harness-run.json` — minimum-size window, 200% text scale through the `A+`
  control, and the fixed rows.
* `frame-report.json` — the app's own opt-in `--performance-report` output.

## cli/

Direct captures from the packaged `aperture-cli.exe`:

* `import-txt-unsupported.txt` — an unsupported practice list is refused, with the
  exact unsupported cards and "never substituted".
* `import-url-malformed.txt` — a non-numeric deck id is refused.
* `import-url-host.txt` — a non-allowlisted host is refused.

## stall/

* `dump-a.txt`, `dump-b.txt` — two live accessibility dumps taken 75 seconds apart
  while the packaged app sat in the D1 deadlock (turn 13 cleanup, digest
  `91a2c0d52cff`, `discarded an AI decision [illegal_choice]`). Same state in both.
