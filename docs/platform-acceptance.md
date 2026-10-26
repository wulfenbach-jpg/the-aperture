# Platform and accessibility acceptance (#30)

This document records what was actually executed, on which packaged revision, with
which input path — and what remains unverified. It exists because a screenshot
proves only that a process rendered: an acceptance claim needs an input, an
observable state change, and a readback.

Release sign-off is **not** claimed here. #30 covers platform/accessibility
journeys; the Commander corpus gate (#17) and the conformance soak (#32) remain
separate release prerequisites.

## Scope of this increment

Executed on Windows only, against the packaged desktop binary from the exact
merged base `51fbc6b2803ef134badd41d408047fc8b339ba72`:

| Item | Status |
| --- | --- |
| Packaged Windows build (portable zip + stamp + manifest) | executed |
| Native input: UIA control actions + window keyboard/mouse messages | executed |
| Human journey: launch → hand → priority → stack → save → load → finish | executed |
| Negative import (unsupported/malformed deck) refusal | executed |
| Failure modes: no model service, failing import URL, unwritable save, stale worker result | executed |
| Minimum window size, 200% text scale, fixed rows | executed |
| Opt-in frame probe (callback duration / interval) | executed |
| Screen reader (NVDA/JAWS/Narrator) pass | **UNVERIFIED — not executed** |
| Physical keyboard / real mouse (human hands on hardware) | **UNVERIFIED — not executed** |
| High-DPI ratios other than the 125% system scale present on this host | **UNVERIFIED — not executed** |
| macOS / Linux packaging | **UNVERIFIED — not executed** |
| GPU present / input-to-photon latency | **UNVERIFIED — not measured** |

## The packaged revision

> **Evidence revision notice.** The recorded journey and defect evidence in this
> document was produced from the packaged build of `51fbc6b`. Master has since
> advanced to `8de0961` (#16 typed counters, creature tokens, keywords and library
> operations), so per the integration owner's coordination note this evidence is
> **not offered as acceptance evidence for current master**. The harness, this
> document and the defect record carry over; the packaged journey must be
> re-produced from a build of `8de0961` or later before the table below is treated
> as the acceptance result. Nothing else in this document changes.

```
scripts\package.cmd
```

produced `dist/the-aperture-0.1.0-51fbc6b/` and its zip from the clean tree at
`51fbc6b`. `BUILD-STAMP.txt` records `commit=51fbc6b2803ef134badd41d408047fc8b339ba72`,
`source_dirty=false`, `rustc 1.97.1`, `target=x86_64-pc-windows-msvc`. The
package's own verifier re-derived all 12 file hashes and a smoke PNG from the
packaged binary before QA started; the harness records the SHA-256 of the exact
`aperture.exe` it drove in every `harness-run.json`.

QA targets that packaged binary — never `target/release`. Every run copies the
unpacked bundle to a scratch journey directory so the package's manifest stays
verifiable while the app writes its save file next to itself.

## How the harness drives the app

`scripts/platform_qa/platform-qa.ps1` (Windows PowerShell 5.1, no external
dependencies) talks to the running window in two ways:

1. **UI Automation** for named controls — the buttons and checkboxes the app
   actually exposes (`Keep this hand`, `Play Forest [CR 305.2, 701.18a]`,
   `Confirm load`, …). Activation uses `InvokePattern`, `TogglePattern` for
   checkboxes, and falls back to focus + Enter or a window-level click.
2. **Window messages** for keys (`WM_KEYDOWN`/`WM_KEYUP`) posted to the exact
   window handle, so the journey uses the documented shortcuts (`P`, `S`, `L`,
   `N`, `I`) rather than reaching into application state.

Text entry into the app's text fields was attempted with both accessibility
`SetValue` and posted `WM_CHAR`, and **neither reached the app's own string** in
this build (the accessibility value changed, the app's state did not, and an
import of the "typed" URL refused it as empty input). The harness therefore does
not type into fields; it uses the packaged CLI and the startup deck path for the
import boundary instead, and says so in the defect notes.

Two details cost real debugging time and are worth recording:

* The process's `MainWindowHandle` is **not** the egui window — winit also owns a
  small hidden helper window, and that is what the property reports. The harness
  resolves the real top-level window through UI Automation by process id.
* egui only republishes its accessibility tree on a frame, so an idle window
  reports a stale (or empty) tree. The harness posts a harmless mouse-move to
  request a repaint before each read.

Nothing in the harness reads application memory, injects code, or asserts on
pixels: it invokes controls, sends keys, and reads back the app's own status
text, header (`... stack N ... digest …`) and the files the app writes.

### Running it

```
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/platform_qa/platform-qa.ps1 `
    -Mode journey  -Exe <unpacked>/aperture.exe -Journey <unpacked> -OutDir evidence/issue30/journey
powershell ... -Mode failures -Exe ... -Journey ... -OutDir evidence/issue30/failures
powershell ... -Mode viewport -Exe ... -Journey ... -OutDir evidence/issue30/viewport -Width 1024 -Height 640
```

Exit code `0` means every executed step passed; a step reported `NOT-EXERCISED`
is not a pass. `-Mode probe` dumps the live accessibility tree and is how the
control names above were discovered.

## Results

See `evidence/issue30/` for the machine-readable record of each run
(`harness-run.json`, per-step UIA tree dumps, the save file the app wrote, and
the frame report). The summary below is derived from those files, not from a
separate narrative.

<!-- RESULTS -->

### Packaged revision

Built from the clean tree at `51fbc6b`, `source_dirty=false`, rustc 1.97.1,
`x86_64-pc-windows-msvc`. The packager's own verifier re-derived 12 file hashes
and rendered a smoke PNG from the packaged binary before QA began. Harness
`aperture.exe` SHA-256 (recorded per run): `104a79e2f06846caee69ef9ab1830b609c97d15a043761a6c18609dac5d3afad`.

### Human journey on the packaged binary (seed 4242, 2 seats, ordinary 20-life)

| Step | Result | What was actually observed |
| --- | --- | --- |
| launch | PASS | requested 1280x720 → window manager made the outer window **1618x947** (125% system DPI); 111 UIA nodes, 38 buttons |
| mulligan | PASS | kept the dealt hand through the `Keep this hand` control (`InvokePattern`) |
| table | PASS | reached an offered main-phase action after **13 real priority passes** (the human is not on the play; P1 acts first) |
| priority/land | PASS | invoked the engine-offered `Play Forest   [CR 305.2, 701.18a]`; header digest `320e0bd4f6d5` → `bc0c83054150` |
| stack cast | PASS | invoked `Cast Runeclaw Bear   [CR 117.1a, 601.2a]` after one extra land drop; header stack count **1** |
| stack resolve | PASS | stack emptied by successive real `P` passes; digest `3ee3fbfc0ca8` → `ba171b04b429` |
| save | PASS | wrote `the-aperture-save.json` (56,423 bytes, save v10, 75 recorded actions, turn 4 precombat main); the live digest was unchanged by the write |
| mutate | PASS | a further real pass moved the position past the save (`ba171b04b429` → `7ceeb335328a`) |
| load | PASS | `Confirm load` restored the saved position: the live digest came back to **`ba171b04b429`** |
| autoplay enable | PASS | both `Let the AI play my seat too` and `AI autoplay` ended in state **On** (read back with `TogglePattern`, not assumed) |
| finish | **FAIL** | no terminal result after 480 s of autoplay. The harness recorded the last engine note verbatim: `WAITING: P1: discard 1 card(s) down to maximum hand size (CR 514.1)` — see defect D1. The app itself did not crash: its captured stderr and stdout were empty for the whole run. |

The load/digest round trip is the strong part of this table: a load that silently
did nothing would leave the digest at `7ceeb335328a`.

### Negative import (the support boundary, executed)

| Probe | Artifact | Observed |
| --- | --- | --- |
| unsupported practice list, native table | `aperture.exe --deck-a unsupported-practice-list.txt` | `DECK REFUSED — deck file … is invalid: line 1: 'Black Lotus' is not in the supported pool. Refusing to substitute the bundled practice deck.` |
| unsupported practice list, CLI | `aperture-cli import-txt --file …` | `LAUNCH REFUSED: unsupported card(s) in the main deck: Black Lotus x1 … Sol Ring x1 …` and `text list refused for play` |
| malformed deck URL | `aperture-cli import --url https://archidekt.com/decks/not-a-deck-id` | `Error: 'not-a-deck-id' is not a numeric Archidekt deck id` (exit 1) |
| non-allowlisted host | `aperture-cli import --url https://example.com/decks/1` | `Error: host 'example.com' is not allowlisted (approved hosts: archidekt.com, www.archidekt.com)` (exit 1) |

This is the difference between bounded practice support and arbitrary Commander,
demonstrated rather than implied.

### Failure modes

| Probe | Result | Observed |
| --- | --- | --- |
| no model service (`--decider`, nothing on 127.0.0.1:8765) | PASS | `decider-2b unreachable (request failed: …; heuristic active)` within ~2 s, and the window kept accepting input afterwards (help panel opened) |
| unwritable save target (a directory squats on the default save path) | PASS | `save failed: Access is denied. (os error 5)` |
| stale background result (a stalled model service, then New game) | PASS (once, not reliably reproduced) | `discarded a stale AI decision [stale_generation] (the game changed)` captured in `evidence/issue30/failures-3/`; later attempts started the new game after the decision had already resolved, so the notice never appeared |

The stale-result probe is honest but flaky: it depends on the worker still being
in flight when the generation changes. It is recorded as passing on one captured
run and not reproduced on later ones, which is weaker evidence than a
deterministic probe would be.

### Viewport, scale and frame probe (1024x640 minimum)

* requested 1024x640 → outer window **1298x847** (the app enforces its minimum at
  the 125% system DPI; 1024x640 logical does not produce a 1024x640 outer rect).
* text scale reached **200%** through the `A+` control, read back from the
  `text 200%` readout.
* at 200% and minimum size the fixed rows stayed reachable: `Pass (P)`,
  `Save (S)`, `Load (L)`, `Zones (Z)`, `Accessibility (A)`, `Help (H)` were all
  present and enabled.
* opt-in frame probe (`--performance-report`, idle heuristic worker, forced 60 Hz
  repaint): callback duration p50 **589 µs**, p95 **852 µs**, max **14,496 µs**
  over 681 samples; inter-callback interval p50 **17,012 µs**, p95 **17,779 µs**.

These are main-thread callback timings. They are **not** GPU present time,
compositor time, or input-to-photon latency, and the report says so itself.


## Defects found and their state

<!-- DEFECTS -->

### D1 — OPEN — AI autoplay livelocks on a cleanup discard, so a game cannot finish

**Severity:** blocks the #30 "finish" criterion on the packaged build.

**Observed on:** `the-aperture-0.1.0-51fbc6b` (packaged), 2 seats, ordinary
20-life, seed 4242, `--autoplay`, both seats AI after `Let the AI play my seat too`.

**Symptom:** the position freezes at cleanup with an outstanding discard-to-hand-size
choice (CR 514.1). The app reports, in its own status area:

```
discarded an AI decision [illegal_choice] (not in the live legal offer)
WAITING: P1: discard 1 card(s) down to maximum hand size (CR 514.1)
· 2 seats · profile practice-20 (20 life) · turn 13 · cleanup step · active P1 · stack 0 · digest 91a2c0d52cff
```

The AI keeps producing a choice for that pending decision, the app keeps rejecting
it because it is not in the seat's live legal offer, and the turn never advances.

**Evidence:** `evidence/issue30/stall/dump-a.txt` and `dump-b.txt` are two live
accessibility dumps taken **75 seconds apart**. Both show turn 13, cleanup step,
digest `91a2c0d52cff`, and the same `illegal_choice` note. The state is not merely
slow: it does not move. A second independent run reached the *same* digests and
the same deadlock (`turn 12 combat damage dfc18cbf9c6a` → `turn 13 cleanup
91a2c0d52cff` → frozen), so the deadlock is reproducible at this seed rather than
a one-off.

**Why it is not fixed here:** the cause is on the AI decision/policy side
(which action it chooses for a pending choice) and in how the UI validates that
choice against the live offer. This lane owns `docs/platform-acceptance.md`,
`scripts/platform_qa/**` and `evidence/issue30/**`, and the work order explicitly
excludes search/policy and model-protocol changes. Changing the AI's choice
behaviour to make the acceptance pass would also be exactly the kind of fix that
hides a real defect behind the test.

**Closure condition:** the exact packaged revision must complete this journey —
autoplay from the same seed reaching a terminal result (win or draw) that the save
file records — before the "finish" line in the results table can read PASS.

### Not defects (recorded so the evidence is not misread)

* An early run appeared to freeze at turn 4 with no AI progress. That was the
  harness toggling `AI autoplay` **off** — the flag `--autoplay` had already turned
  it on, and a blind toggle flipped it back. It is a harness bug, fixed by reading
  `TogglePattern` state before toggling; it is not a product defect.
* `import URL` typed with window character messages into the import panel did not
  reach the app's text state (the field read back through the accessibility tree
  was not the app's buffer). That is a harness limitation; the import boundary was
  therefore exercised through the packaged CLI and through the startup deck path,
  which are both real packaged code paths.


## What this does not claim

* No screen-reader pass, no physical-keyboard run, no non-Windows platform, no
  GPU/present timing, no input-to-photon latency.
* The terminal phase of the journey is reached with the app's built-in AI
  autoplay driving both seats, after the human-driven priority/stack/save/load
  portion. That is disclosed in the per-step evidence rather than presented as a
  hand-played finish.
* A bounded practice corpus, not Commander: the negative import below is the
  boundary evidence, not a workaround.
