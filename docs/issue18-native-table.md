# Native four-seat practice table (issue 18)

`aperture.exe` is the native desktop table. This build exposes the existing
2–4-seat engine directly; it does not reimplement rules in the UI.

## Launch

```
launch.cmd                         :: builds if needed and starts the table
cargo run --release -p aperture-ui
```

Command-line options:

```
--seats 2|3|4                      initial table size (default 2)
--profile ordinary|bounded         ordinary = 20 life; bounded = 40 life with
                                   one supported Isamaru per seat
--seed N                           deterministic game seed
--deck-a PATH / --deck-b PATH      explicit P0/P1 lists (never substituted)
--ai-seat 0|1                      two-seat only, which seat the heuristic AI plays
--autoplay / --ai-both             keep the AI moving without clicks
--size WxH                         initial window size (e.g. 1280x720; min 1024x640)
--smoke OUT.png                    capture the real window and exit
```

The in-app **TABLE** and **PROFILE** controls start a new table. **New game**
keeps the selected seat count and profile; it never falls back to two seats.
Ordinary practice defaults to the alternating compiled Temur/Izzet lists. The
bounded command default gives every seat the compiled
`decks/command-zone-practice.txt` library (white sources plus the supported
command-zone cards) so the supplied Isamaru can be cast; explicit
`--deck-a`/`--deck-b` or imported lists still replace defaults exactly.

## What is on screen

- A compact 2x2 seat panel grid (the human is P0; other seats are heuristic AI).
  Each panel shows life, hand/library counts, every public zone, commander
  identity/tax, public commander damage, and the seat's battlefield.
- `ACTIVE`, `PRIORITY`, `PENDING` and `ELIMINATED` status markers; the status
  strip also shows the pending owner and turn/step/stack/digest.
- Your own hand, plus a persistent **YOUR ACTIONS** strip above the scrolling
  table. Pending choices, legal actions, target picks, activated abilities,
  combat declarations and commander replacement/return choices all appear there
  and go through the engine's authoritative submission check.
- Opponent hands and libraries are counts only. The renderer reads card data
  from the masked engine `PlayerView`; it never receives opponent hand or
  library contents.
- **Zones (Z)** lists every seat's graveyard, exile and command zone. Click a
  battlefield card or a zone card to inspect it; the Inspect window is
  read-only.
- **Import (I)** certifies one list per seat. *Start imported match* is enabled
  only when every seat is certified; the exact planned lists are applied with
  no substitutions. The old two-seat import remains the same path with two
  seats.
- **Save (S) / Load (L)** work for ordinary 20-life and bounded 40-life games,
  including 3/4-seat games. Load restores the seat count, profile, commanders,
  pending choice, RNG position and AI settings.

## Keyboard and zoom

- `Tab` / `Shift+Tab` move focus; `Enter` activates the focused control.
- `P` pass · `N` new game (retains seats/profile) · `S` save · `L` load ·
  `I` import · `Z` zones · `K` sandbox · `C` coaching · `H` help ·
  `+`/`−` text zoom · `Esc` cancels a target pick.
- Shortcuts are ignored while a text field has focus. `A−`/`A+` buttons do the
  same zoom without the keyboard.

## Honest bounds

- The **bounded 40 life** option is one supported Isamaru per seat in the
  command zone with the existing command-zone/tax/combat-damage behavior. It is
  practice, **not** Commander: no 100-card construction, color identity, legend
  rule, partners or full APNAP cleanup.
- Sandbox `+1/+1` counters are stored and displayed on the card, but they are
  **not** a rules-supported counter system; combat math ignores them.
- This is the existing bounded 28-card practice pool. Unsupported imports are
  refused rather than substituted.
- Combat damage assignment remains lethal-first automatic (no manual divide).

## Packaging

`dist/aperture-issue18-native-table.zip` contains fresh release
`aperture.exe`/`aperture-cli.exe`, the bundled `decks/`, `LAUNCH.cmd` and a
short scope README. No model weights or private configuration are included.
