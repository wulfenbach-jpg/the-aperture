# Evidence — #30 platform and accessibility acceptance

Machine-recorded output from `scripts/platform_qa/platform-qa.ps1` runs against the
packaged Windows build `the-aperture-0.1.0-60b89b5` (merged master `60b89b5`,
which carries the D1 fix from #34). Nothing here is hand-written except this index:
the numbers in `docs/platform-acceptance.md` come from these files.

Provenance: every `harness-run.json` names the harness script and its working-tree
SHA-256, the exact `aperture.exe` it drove
(`c1372dd036af4fd2b89532b83b1746b6482ed0fa079765030c79373a59906f65`), the table
size (`seats: 4`), and its own `started`/`finished` timestamps.

* `journey/`, `failures/` and `viewport/` carry `harness_sha256 = f105334a3072…`,
  the harness as committed at `1912ac8`; reproduce it with
  `git show 1912ac8:scripts/platform_qa/platform-qa.ps1 | sha256sum -` (the
  recorded value is the CRLF working-tree hash this checkout actually ran from).
* `import/` carries `harness_sha256 = 597dda096a09…`, the current committed
  harness, which differs only in that mode's own seat-count predicate (the header
  reads `· 4 seats ·`, so the earlier `seats (\d+)` parse scored a table that had
  really started as `failed`).

Records from any other harness revision, and the earlier records whose `started`
was the save time, are not kept here.

Absolute paths inside the records point at the scratch directory the run used
(`…/cache/scratch/issue30-60b89b5a/pkg`); the files themselves are committed under
the directories below.

## package/

* `BUILD-STAMP.txt` — the stamp inside the QA target: `commit=60b89b5…`,
  `source_dirty=false`, rustc 1.97.1, `x86_64-pc-windows-msvc`, `signed=false`.
* `MANIFEST.sha256` — the 12-file manifest the packaged verifier re-derived.
* `verify-package.txt` — the verifier's own capture: 12/12 hashes re-derived, the
  stamp confirmed through the packaged CLI, and `aperture.exe --smoke` producing a
  450,034-byte PNG. Ends with `PACKAGE VERIFIED`.

## journey/

One run of the scripted native-input journey on a four-seat table
(launch → hand → priority → land → stack → save → load → AI-vs-AI to a terminal
result → packaged-CLI replay), 09:57:33 → 10:14:08, **14/14 steps passed**.

* `harness-run.json` — every step with its status, the observed text, the sampled
  autoplay positions, and the full transcript; includes the SHA-256 of the exact
  `aperture.exe` and of the harness.
* `journey-state.json` — the session values the steps compared (requested vs
  actual window, header digests at save/load, the save-file digests, the terminal
  save, the autoplay sampling).
* `01-launch-tree.json`, `03-table-tree.json`, `04-stack-tree.json`,
  `05-resolved-tree.json`, `06a-confirm-tree.json`, `06-loaded-tree.json`,
  `07-terminal-tree.json`, `final-tree.json` — the live accessibility trees at the
  points where the journey changed state.
* `the-aperture-save-midgame.json` — the file the packaged app wrote at the save
  step (149,145 bytes, `save v10`, turn 8 PrecombatMain, 310 recorded actions, no
  outcome). Copied straight from the run's save path by the harness.
* `the-aperture-save-terminal.json` — the file the same instance wrote at the end
  (`save v10`, 1,534 recorded actions, turn 38, outcome
  `{"Win":{"winner":0,…}}`, content digest `3f574cca93c0…`). This is the artifact
  the replay step re-runs.
* `cli-*.out.txt` / `cli-*.err.txt` — the packaged CLI's replay of the finished
  game: `stored digest 3f574cca93c0 | recomputed 3f574cca93c0 | matches`, 1,534
  actions, 3,826 events, `reproduces the save`.
* `app-*.out.txt`, `app-*.err.txt` — the packaged app's own streams for the run;
  both empty.

## failures/

One full run of the failure-mode probes on the same package, 10:16:13 → 10:16:53:
**8 passed, 1 not-exercised**.

* `harness-run.json` — no model service, unsupported-list CLI report, malformed
  URL, non-allowlisted host, native-table refusal, unwritable save, stale
  background result, offline practice, and the network-off row marked
  `not-exercised`.
* `cli-*.out.txt` / `cli-*.err.txt` — raw stdout/stderr of every packaged-CLI
  invocation, with the real exit code recorded in the step's `data.exit`.
* `app-*.out.txt` / `app-*.err.txt` — the app's own streams for each launch.
* `10-unsupported-deck-ui.json`, `12-unwritable-save.json`,
  `13-stale-result.json` — accessibility trees captured at the refusal, the failed
  save and the stale-result notice.

## import/

The positive import → start leg, 10:20:18 → 10:20:23: **2 passed, 2 not-exercised**.

* `harness-run.json` — the packaged CLI accepting a supported list as playable
  (`launchable=true`, `unsupported_cards=0`) and the packaged app starting a real
  four-seat table from that same list; the Archidekt-URL and slow-import rows are
  recorded as `not-exercised` with their reasons.
* `cli-*.out.txt` / `cli-*.err.txt` — the import-txt report the first step asserts on.
* `30-import-start-tree.json`, `final-tree.json` — the mulligan offer and the table
  that followed, read back from the live window.

## viewport/

One run at 1024x640 logical, 10:16:59 → 10:17:11: **3/3 passed**.

* `harness-run.json` — minimum-size window (1298x847 outer at 125% DPI), 200% text
  scale through `A+`, the six fixed rows, and the frame probe.
* `frame-report.json` — the app's own opt-in `--performance-report` output
  (callback p50 509 µs / p95 731 µs / max 12,208 µs over 687 samples).
* `20-zoom-tree.json`, `21-frame-tree.json`, `final-tree.json`, plus the app's
  stream captures for the run.
