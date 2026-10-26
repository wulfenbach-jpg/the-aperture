# Platform and accessibility acceptance (#30)

This document records what was actually executed, on which packaged revision, with
which input path — and what remains unverified. It exists because a screenshot
proves only that a process rendered: an acceptance claim needs an input, an
observable state change, and a readback.

The "… → finish" criterion that this card exists for is **met** on the packaged
merged-master build: a four-seat AI-vs-AI game reached a terminal result in the
real window, and the packaged CLI re-ran the recorded action sequence to the same
digest. Release sign-off is still **not** claimed here: the Commander corpus gate
(#17) and the conformance soak (#32) remain separate release prerequisites.

## Scope of this increment

Executed on Windows only, against the packaged desktop binary built from the
exact merged base `60b89b54f2d930292533e12c70c38b1dba7933d2` (which carries the
D1 fix, #34):

| Item | Status |
| --- | --- |
| Packaged Windows build (portable zip + stamp + manifest) | executed — `PACKAGE VERIFIED` |
| Native input: UIA control actions + window keyboard messages | executed |
| Criterion 1: positive import → start (supported list) | executed — packaged CLI import accepted, packaged app started a 4-seat table from that list |
| Criterion 1: launch → hand → priority → land → stack → save → load | executed |
| Criterion 1: finish — four-seat AI-vs-AI game to a terminal result | **executed — PASS** |
| Criterion 1/4: matching replay of the finished game | **executed — PASS** (packaged CLI re-runs the recorded line) |
| Criterion 3: negative import — native table refusal | executed |
| Criterion 3: negative import — packaged CLI refusals with captured exit codes | executed |
| Criterion 3: no model service, unwritable save, stale worker result | executed |
| Criterion 3: offline use | practice path executed with no service and no import; a hard network-off capture is **NOT EXERCISED** (see below) |
| Criterion 3: slow import | **NOT EXERCISED** — no way to slow the allowlisted public host from this fixture |
| Criterion 1: positive Archidekt URL import | **NOT EXERCISED** — needs live egress plus a deck wholly inside the bounded pool |
| Typing an import URL into the app's own field | **NOT EXERCISED** — assistive text entry is unverified, not "not a defect" (see below) |
| Criterion 2: minimum window size, 200% text scale, fixed rows | executed |
| Criterion 2: opt-in frame probe (callback duration / interval) | executed |
| Criterion 2: screen reader (NVDA/JAWS/Narrator) pass | **UNVERIFIED — not executed** |
| Criterion 2: physical keyboard / real mouse (human hands on hardware) | **UNVERIFIED — not executed** |
| Criterion 2: high-DPI ratios other than the 125% system scale on this host | **UNVERIFIED — not executed** |
| macOS / Linux packaging | **UNVERIFIED — not executed** |
| GPU present / input-to-photon latency | **UNVERIFIED — not measured** |

## The packaged revision

```
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package.ps1
```

run in a clean checkout of merged master `60b89b5` produced
`dist/the-aperture-0.1.0-60b89b5/` and its zip. `BUILD-STAMP.txt` records
`commit=60b89b54f2d930292533e12c70c38b1dba7933d2`, `source_dirty=false`,
`rustc 1.97.1`, `target=x86_64-pc-windows-msvc`, `scope=bounded-28-card-practice`,
`signed=false`. The bundle's own verifier re-derived all 12 file hashes, confirmed
`contents match MANIFEST.sha256`, confirmed the stamp through the packaged CLI,
and launched the packaged desktop build in `--smoke` mode to a real PNG:

```
PACKAGE VERIFIED: 12 file hashes re-derived, contents match MANIFEST.sha256
```

`evidence/issue30/package/verify-package.txt` is that capture; the bundle's
`BUILD-STAMP.txt` and `MANIFEST.sha256` are committed beside it.

The QA target is the **merged-master** binary, not a binary built from this
branch: this branch changes only `docs/`, `scripts/platform_qa/` and `evidence/`
— no product source — so the stamped commit and the binary under test are the
merged tree a user would actually get.

The exact artifacts QA drove (recorded as `exe_sha256` in every
`harness-run.json`):

| Artifact | SHA-256 |
| --- | --- |
| `aperture.exe` | `c1372dd036af4fd2b89532b83b1746b6482ed0fa079765030c79373a59906f65` |
| `aperture-cli.exe` | `ec25aadff0f9fcbaa99cc38a1c17b7a46c83211bbee440d6358062138529d79e` |
| package zip | `70355bcabc00c461ecc8501e4ab8872442c4ecd3a14e759e172701b39ec4afe3` |

Every run works on a copy of the unpacked bundle in a scratch directory, so the
package's manifest stays verifiable while the app writes its save file next to
itself. `target/release` is never the QA target.

## Evidence provenance

Every record in `evidence/issue30/` was written by the **committed** harness in
this same change: `scripts/platform_qa/platform-qa.ps1`, working-tree SHA-256
`f105334a30723a0984211cd03f92411cefea9301f8a8052259ab1bee9dc6ec90`. Each
`harness-run.json` names the script path and that hash, the exact `aperture.exe`
hash, the table size (`seats: 4`), and its own real `started`/`finished`
timestamps (the run start, not the save time — that was a round-1 review finding).

Git stores `.ps1` text with LF and checks it out with CRLF under
`core.autocrlf=true`, so the recorded hash is the bytes the harness actually ran
from on this checkout; `git hash-object scripts/platform_qa/platform-qa.ps1`
names the committed blob for the same file.

What each run wrote, and where it landed:

| Record | Mode | `started` → `finished` | Result |
| --- | --- | --- | --- |
| `journey/harness-run.json` | journey (4 seats) | 09:57:33.031 → 10:14:08.941 | 14/14 passed |
| `failures/harness-run.json` | failures | 10:16:13.414 → 10:16:53.967 | 8 passed, 1 not-exercised |
| `import/harness-run.json` | import | 10:20:18.109 → 10:20:23.305 | 2 passed, 2 not-exercised |
| `viewport/harness-run.json` | viewport | 10:16:59.912 → 10:17:11.987 | 3/3 passed |

One revision note, stated rather than glossed: the `import` record's
`harness_sha256` is `597dda096a09f8d3…`, the harness **after** a one-line fix to
that mode's own pass predicate (the header prints the table size *before* the word
— `· 4 seats ·` — and the first version parsed `seats 4`, so a table that really
started was scored `failed`). That fix touches only `Invoke-Import`, which no other
mode calls; the other three records carry `f105334a3072…`, the committed revision
at `git log` commit `1912ac8`, so their hashes are reproducible with
`git show 1912ac8:scripts/platform_qa/platform-qa.ps1 | sha256sum -`.

All four record `exe_sha256 = c1372dd0…` (the packaged `60b89b5` binary). Raw
captures the harness itself wrote beside each record
(`app-*.out.txt` / `app-*.err.txt` for the app's own streams, `cli-*.out.txt` /
`cli-*.err.txt` for each packaged-CLI invocation) are committed unedited.

## How the harness drives the app

`scripts/platform_qa/platform-qa.ps1` (Windows PowerShell 5.1, no external
dependencies) talks to the running window in two ways:

1. **UI Automation** for named controls — the buttons and checkboxes the app
   actually exposes (`Keep this hand`, `Play Forest [CR 305.2, 701.18a]`,
   `Cast Runeclaw Bear [CR 117.1a, 601.2a]`, `Confirm load`, …). Activation uses
   `InvokePattern`, `TogglePattern` for checkboxes, and falls back to focus +
   Enter or a window-level click.
2. **Window messages** for keys (`WM_KEYDOWN`/`WM_KEYUP`) posted to the exact
   window handle, so the journey uses the documented shortcuts (`P`, `S`, `L`,
   `N`, `I`) rather than reaching into application state.

Nothing in the harness reads application memory, injects code, or asserts on
pixels: it invokes controls, sends keys, and reads back the app's own status
text, header (`… stack N … digest …`) and the files the app writes.

Text entry into the app's text fields is **unverified**: accessibility `SetValue`
and posted `WM_CHAR` were both attempted against this and the previous build, and
neither reached the app's own string (the accessibility value changed, the app's
state did not). That is recorded as an assistive-technology text-entry gap — the
same standing as the screen-reader row — rather than dismissed as a harness quirk,
and the import boundary is therefore driven through the packaged CLI and the
startup deck path, which are real packaged code paths.

Two details cost real debugging time and are worth recording:

* The process's `MainWindowHandle` is **not** the egui window — winit also owns a
  small hidden helper window, and that is what the property reports. The harness
  resolves the real top-level window through UI Automation by process id.
* egui only republishes its accessibility tree on a frame, so an idle window
  reports a stale (or empty) tree. The harness posts a harmless mouse-move to
  request a repaint before each read.

### Running it

```
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/platform_qa/platform-qa.ps1 `
    -Mode journey  -Exe <unpacked>/aperture.exe -Journey <unpacked> -OutDir evidence/issue30/journey -Seats 4
powershell ... -Mode failures -Exe ... -Journey ... -OutDir evidence/issue30/failures
powershell ... -Mode import   -Exe ... -Journey ... -OutDir evidence/issue30/import -Seats 4
powershell ... -Mode viewport -Exe ... -Journey ... -OutDir evidence/issue30/viewport -Width 1024 -Height 640
```

Exit code `0` means every executed step passed; a step reported `NOT-EXERCISED`
is not a pass and does not fail the run. `-Mode probe` dumps the live
accessibility tree and is how the control names above were discovered.

`journey` mode drives a four-seat table (`-Seats`, default 4): the harness holds
P0 until the human-side choices are proven, then sets both `Let the AI play my
seat too` and `AI autoplay` (state read back with `TogglePattern`) so all four
seats play the game out. While autoplay runs it samples the live header every
15 s, so the record shows the position **moving** (53 distinct sampled positions
across the finished run, as counted in `journey/journey-state.json`) instead of
just asserting a wall-clock wait. If no terminal result arrives within
`-FinishTimeoutSec` (default 900 s) the step fails and the harness writes two
timestamped dumps of the same process 75 s apart
(`stall/dump-a.txt`, `stall/dump-b.txt`) and reports whether the position moved.

`import` mode executes the positive import path: the packaged CLI imports a
supported text list and must report it playable, and the packaged app must then
start a real table from that same list.

## Results

The numbers below are transcribed from `evidence/issue30/**`, not from a separate
narrative. Three of them are properties of a run rather than of the build, and do
differ between runs on this same package: the UIA node count (**134** here), the
autoplay duration in seconds (**821** here) and the number of sampled positions
(**53** here). Each is quoted from that run's own `journey/harness-run.json` and
`journey/journey-state.json` (`autoplay_seconds`, 53 progress entries), which are
the authoritative files; an independent re-run of the same package recorded 135
nodes, 815.4 s and 52 samples. No acceptance claim below depends on any of the
three — they are reported because the record contains them.

### Journey on the packaged binary (seed 4242, four seats, ordinary 20-life)

| Step | Result | What was actually observed |
| --- | --- | --- |
| launch | PASS | requested 1280x720 → window manager made the outer window **1618x947** (125% system DPI); 134 UIA nodes on a 4-seat table |
| mulligan | PASS | kept the dealt hand through the `Keep this hand` control (`InvokePattern`) |
| table | PASS | reached an offered main-phase action after **33 real priority passes** (the human seat is not on the play); header digest `03e29ff44952` |
| priority/land | PASS | invoked the engine-offered `Play Forest [CR 305.2, 701.18a]`; header digest `03e29ff44952` → `a4af239af81d` |
| stack cast | PASS | invoked `Cast Runeclaw Bear [CR 117.1a, 601.2a]` after one extra land drop; header stack count **1** |
| stack resolve | PASS | stack emptied by successive real `P` passes; header digest `2aac9619e929` → `f6a40af635b9` |
| save | PASS | wrote 149,145 bytes (save v10, 310 recorded actions, turn 8 PrecombatMain); the live header digest stayed `f6a40af635b9` |
| mutate | PASS | a further real pass moved the position past the save (`f6a40af635b9` → `0f954215d14d`) |
| load | PASS | `Confirm load` restored the saved position: the live header digest came back to **`f6a40af635b9`** |
| autoplay enable | PASS | both `Let the AI play my seat too` and `AI autoplay` ended in state **On** (read back with `TogglePattern`, not assumed) |
| finish | **PASS** | terminal text `P0 wins — opponents' life totals reached 0 or less (CR 704.5a)` after **821 s** of four-seat AI-vs-AI autoplay (53 distinct sampled positions); the terminal save records `{"Win":{"winner":0,"reason":"opponents' life totals reached 0 or less (CR 704.5a)"}}`, **1,534 actions**, turn **38**, digest **`3f574cca93c0`** |
| replay | **PASS** | packaged CLI replay of the finished game: exit 0, stored digest `3f574cca93c0` = recomputed `3f574cca93c0`, 4 seats, 1,534 actions, 3,826 events, "reproduces the save" |
| finish driver | PASS (declared) | the terminal phase was driven by the built-in AI autoplay on every seat; the harness declares that rather than implying a hand-played finish |
| focus untouched | PASS | the foreground window title was unchanged before and after the whole journey |

Two different digests appear in the save rows, and they mean different things:

* `f6a40af635b9` is the **live position digest** the app prints in its header, and
  the one the load round trip compares. It is identical before the save and after
  the load, and different after the intermediate pass — that is what makes the
  round trip evidence rather than a coincidence.
* the save file's own `digest` field is the **content digest of the save**, not a
  position digest, and is not compared across the load. The mid-game file the
  packaged app wrote at the save step is committed verbatim as
  `journey/the-aperture-save-midgame.json`; the terminal file is
  `journey/the-aperture-save-terminal.json`. The harness copies both straight
  from the run's own save path, so neither depends on a sibling run.

The load/digest round trip is the strong part of this table: a load that silently
did nothing would have left the digest at `0f954215d14d`.

The app's own streams are committed as `journey/app-*.out.txt` and
`journey/app-*.err.txt` (empty for the whole run). The game reached its result
without a crash, a panic or a livelock.

### Matching replay (the finished game, not just a saved file)

`aperture-cli replay --file <terminal save>` re-runs the recorded action sequence
from the seed and the deck list, and compares the digest it reaches with the one
the file stored:

```
save C:\...\the-aperture-save.json — 4 seat(s): P0 'Temur Beats' vs P1 'Izzet Control' vs P2 'Temur Beats P2' vs P3 'Izzet Control P3'
  stored digest 3f574cca93c0 | recomputed 3f574cca93c0 | matches
  turn 38 step combat damage step | pending None | sandbox modified: false
  ai: human P0; AI seats P1, P2, P3 = deterministic baseline heuristic; autoplay on
  replay of 1534 action(s) -> digest 3f574cca93c0 | events 3826 | reproduces the save
```

The capture is `journey/cli-*.out.txt`. This is what makes "finished" a property
of the game rather than of the window: the same packaged revision, given the
recorded line, arrives at the same position.

### Positive import → start (executed)

| Step | Artifact | Observed |
| --- | --- | --- |
| supported list imported | `aperture-cli import-txt --file decks/izzet-control.txt --json` | exit **0**, `launchable=true`, `unsupported_cards=0`, printed `text list is playable as-is` |
| that list starts a table | `aperture.exe --seats 4 --deck-a decks/izzet-control.txt --autoplay` | mulligan offer appeared, `Keep this hand` invoked, table reached with header `· 4 seats · profile practice-20 (20 life) …` and P0's hand dealt from the imported list: `Island`, `Divination`, `Hill Giant`, `Consider` — none of which appears in the bundled practice deck A (`decks/temur-beats.txt`), so the hand is the proof the list was used. The seat label still reads `Temur Beats` even though `--deck-a` was supplied, because `crates/ui/src/app.rs` hardcodes it; the label is not evidence either way |

Both captures are in `import/` (`cli-*.out.txt`, `30-import-start-tree.json`,
`harness-run.json`).

### Negative import boundary (executed)

| Probe | Artifact | Observed | Exit |
| --- | --- | --- | --- |
| unsupported practice list, native table | `aperture.exe --deck-a unsupported-practice-list.txt` | `DECK REFUSED — deck file … is invalid: line 1: 'Black Lotus' is not in the supported pool. Refusing to substitute the bundled practice deck.` | n/a (UI) |
| unsupported practice list, CLI report | `aperture-cli import-txt --file … --json` | `launchable=false`, `unsupported_cards=2`, two `Unsupported` entries, and the printed line `text list refused for play: … unsupported cards are refused, never substituted` | **0** |
| malformed deck URL | `aperture-cli import --url https://archidekt.com/decks/not-a-deck-id --plan` | `Error: 'not-a-deck-id' is not a numeric Archidekt deck id` | **1** |
| non-allowlisted host | `aperture-cli import --url https://example.com/decks/1 --plan` | `Error: host 'example.com' is not allowlisted (approved hosts: archidekt.com, www.archidekt.com)` | **1** |

`import-txt` is a **reporting** command: its refusal is the `launchable=false`
report field plus the printed "refused for play" line, and it exits 0 by design.
An earlier version of this document claimed `exit 1` for it; that claim was wrong
and the harness now asserts the report's own refusal semantics while recording the
real exit code. The two URL refusals are checked as refusals with the exit code
included: the step passes only when the message matches **and** the process
returns non-zero.

This is the difference between bounded practice support and arbitrary Commander,
demonstrated rather than implied.

### Failure modes

| Probe | Result | Observed |
| --- | --- | --- |
| no model service (`--decider`, nothing on 127.0.0.1:8765) | PASS | `decider-2b unreachable (request failed: …; heuristic active)` and the window kept accepting input afterwards (help panel opened) |
| unwritable save target (a directory squats on the default save path) | PASS | `save failed: Access is denied. (os error 5)` |
| stale background result (a stalled model service, then New game) | PASS | `discarded a stale AI decision [stale_generation] (the game changed)` |
| offline practice (no decider configured, no import used) | PASS | the whole practice path ran with no network surface available to it and every step worked |
| hard network-off (adapter down / no route) | **NOT-EXERCISED** | this QA host offers no isolated network namespace to the harness and disabling its NIC is not available to a scripted run, so the row above records service-absence, not interface-down |
| slow import | **NOT-EXERCISED** | import egress is pinned to the allowlisted `archidekt.com` host with no local override, so a slow or stalled import cannot be produced here without network control; the slow-service and stale-result paths ARE executed |
| positive Archidekt URL import | **NOT-EXERCISED** | needs live egress to the allowlisted host *and* a deck wholly inside the bounded 28-card pool; this fixture has neither, so only the refusals are executed against the URL path |

The stale-result probe passes on this package, but it is timing-dependent rather
than deterministic: the notice appears only if the worker is still in flight when
the generation changes.

### Viewport, scale and frame probe (1024x640 minimum)

* requested 1024x640 → outer window **1298x847** (the app enforces its minimum at
  the 125% system DPI; 1024x640 logical does not produce a 1024x640 outer rect).
* text scale reached **200%** through the `A+` control, read back from the
  `text 200%` readout after every press.
* at 200% and minimum size the fixed rows stayed reachable: `Pass (P)`,
  `Save (S)`, `Load (L)`, `Zones (Z)`, `Accessibility (A)`, `Help (H)` were all
  present and enabled.
* opt-in frame probe (`--performance-report`, idle heuristic worker, forced 60 Hz
  repaint): callback duration p50 **509 µs**, p95 **731 µs**, max **12,208 µs**
  over 687 samples; inter-callback interval p50 **17,000 µs**, p95 **17,515 µs**
  (`viewport/frame-report.json`).

These are main-thread callback timings. They are **not** GPU present time,
compositor time, or input-to-photon latency, and the report says so itself
(`input_frame_render_us` is explicitly labelled "NOT OS/GPU input-to-photon
latency").

## Defects found and their state

### D1 — CLOSED — AI autoplay livelocked on a cleanup discard, so a game could not finish

**Found by this card** on packaged `51fbc6b` and reproduced on packaged `8de0961`
(`aperture.exe d34c991b…`): with every seat on AI autoplay the game froze at a
turn-13 cleanup with an outstanding discard-to-hand-size choice (CR 514.1). The
app reported `discarded an AI decision [illegal_choice] (not in the live legal
offer)` and `WAITING: P1: discard 1 card(s) down to maximum hand size`, and two
dumps of one process 75.6 s apart carried an identical header (`turn 13 · cleanup
step · digest 268f6dbba6dd`). The app streamed nothing: a livelock, not a panic.
That evidence is preserved in this card's run history and in the previous
revision of this document; it does not reproduce on the current package because
the defect is gone.

**Cause:** `legal_actions()` offered exactly one `Pending::Discard` action (a
sampled set) while the heuristic proposed its own worst-cards set; the UI
pre-check compared for equality against that single offer and dropped the
decision as `illegal_choice` without re-requesting, so the table waited forever.

**Fixed in** #34 / PR #55, merged as `60b89b5` ("make the cleanup discard a real,
non-stalling choice"). That lane owed its own RED/GREEN proof; this card verifies
the outcome on the package rather than in a Cargo worktree.

**Closure evidence (this run):** the same packaged revision now reaches a terminal
result — four seats, AI autoplay on every seat, `P0 wins` at turn 38, digest
`3f574cca93c0`, 1,534 actions, 821 s — and the packaged CLI replays that line to
the same digest. The closure condition written into the previous revision of this
document ("the exact packaged revision must complete this journey — autoplay from
the same seed reaching a terminal result that the save file records") is
satisfied.

### Not defects (recorded so the evidence is not misread)

* An early run appeared to freeze at turn 4 with no AI progress. That was the
  harness toggling `AI autoplay` **off** — `--autoplay` had already turned it on,
  and a blind toggle flipped it back. A harness bug, fixed by reading
  `TogglePattern` state before toggling; not a product defect.
* The first attempt at the positive import leg started the app without
  `--autoplay`, so the three AI seats correctly waited at their own opening
  mulligan decisions and P0's offer never arrived within the timeout. That is the
  table behaving correctly, not a defect; the leg now starts the seats on
  autoplay as well.
* `Start-Process -PassThru` did not populate `ExitCode` for a redirected child on
  this host's PowerShell 5.1, which is why earlier records printed `exit ;`. The
  helper now drives the CLI through `System.Diagnostics.Process`, and every
  non-zero expectation above is captured from the process itself.

## What this does not claim

* No screen-reader pass, no physical-keyboard run, no non-Windows platform, no
  GPU/present timing, no input-to-photon latency, no DPI ratio other than the
  125% system scale on this host.
* No assistive-technology text entry: typing into the app's fields is unverified.
* No claim that the terminal phase was hand-played: it was played by the app's
  built-in AI autoplay on all four seats, declared in a `finish-driver` step.
* No release sign-off: the Commander corpus gate (#17) and the conformance soak
  (#32) are still prerequisites, and this card's evidence covers one host.
* A bounded practice corpus, not Commander: the negative import above is the
  boundary evidence, not a workaround.
