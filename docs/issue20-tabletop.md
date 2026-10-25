# Issue 20 — tabletop-grade controls, one searchable zone inspector, honest sandbox

This is bounded practice on the real native four-seat table (79 supported cards
after the issue-17 starter package), not
Commander, not a trainer and not v1. It adds the manual table furniture a player
needs while practising: one searchable zone inspector, explicit table controls,
an explicitly labelled sandbox that taints the session, an honest undo, and
confirmations for every destructive action.

## 1. One searchable zone inspector

`Z` opens **Zone inspector — searchable, viewer-masked**. It is built from the
engine's masked `PlayerView` for the human seat and from nothing else:

* search matches name, printed rules text or mana cost, case-insensitively;
* filters narrow by seat (P0..P3) and by zone (hand, battlefield, graveyard,
  exile, command);
* every seat's counts are listed, and each count is a button that filters the
  inspector to that seat and zone;
* a card's zone comes from `PlayerView::zone_of`, so the shared inspector never
  hardcodes a location: a command-zone card reports `command zone`, and a card
  that is not projected at all reports *no* zone instead of inventing one;
* opponent hands and every library are counts only. They cannot match, and the
  inspector says so rather than showing an unexplained empty list.

`docs/issue18-native-table.md` remains the reference for the four-seat layout and
the fixed primary strip. The window minimum eframe actually enforces is
**1024×640** (`crates/ui/src/main.rs`); 1280×720 is the acceptance viewport. Both
sizes are exercised against `render_ui` for the inspector, the sandbox panel and
the primary strip — the inspector is a floating window and does not change the
persistent strip at either size.

## 2. Manual table controls (all sandbox overrides)

Open the sandbox panel with `K`. Every control there is an **override**, never a
legal-match action. Each one:

* runs through one deterministic engine path (`Game::sandbox`) with no random
  draws, so it cannot disturb the RNG position;
* emits exactly one typed, viewer-aware `Event::SandboxOverride` trace event;
* bumps the engine revision and decision serial, so an outstanding submission
  or a background AI decision computed against the old position is stale;
* is refused with a reason while a rules choice is pending, mid-resolution, on
  an empty or too-small library, or when a commander is on top of the library
  (its CR 903.9b zone-change choice is a rules decision, not an override).

| control | effect | visibility |
| --- | --- | --- |
| Draw 1 | top of your library to your hand | your draw is yours; opponents see "draws a card" |
| Mill 1 | top card to your graveyard, no dies trigger (CR 701.13a) | public (graveyard) |
| Untap all | untap your permanents; **not** an untap step, so summoning sickness is unchanged (CR 502) | public |
| Scry 1 · keep on top / to bottom | look at the top card; keep it or put it on the bottom in the same order (CR 701.22a) | the look is private to your seat |
| Surveil 1 · keep on top / to graveyard | look at the top card; keep it or put it into the graveyard (CR 701.25a) | the look is private to your seat |
| Reveal top 3 | show the top three cards to every seat | public by construction |
| Counter editor | set / add / clear +1/+1 counters on one of your permanents (0..=99) | public; **storage and display only** — this engine's combat math does not apply counters |
| Life editor | set any seat's life total (1..=999) | public |

A sandbox life edit never ends the game: 0 or less is refused, because ending a
game is what a legal match does (CR 704.5a), not an override.

## 3. Legal match versus sandbox: the taint

The record is typed and durable:

* `SandboxRecord { edits: Vec<SandboxEdit> }` in `crates/core/src/sandbox.rs`,
  where each edit carries its kind, seat, turn, step and public description;
* the record is stored in `SessionSave.sandbox`, so it survives save/load;
* `SessionSave::validate` **refuses** a save that carries overrides but claims
  `sandbox_modified: false`, so clearing one boolean by hand cannot launder a
  sandbox game into a match result;
* every result surface uses the same sentence
  (`SandboxRecord::record_line`): *SANDBOX-MODIFIED — … this is NOT a legal
  match result (n sandbox override(s) [...])*. The header strip shows the same
  banner for the rest of the session, and a new game resets the record with the
  position;
* a tainted save is still loadable (the position is the player's), but its
  embedded replay is no longer certified and the UI never claims otherwise.

Legal-match actions are untouched: they stay in the persistent **YOUR ACTIONS**
strip, are validated by the engine's normal transactional path, and only the
engine's own legal-action generator can offer them.

## 4. Undo and study honesty

`Undo (U)` removes the most recent accepted action by **rebuilding** the
position from the deterministic recorded prefix (`Replay::branch_at` +
`run_recorded`), not by patching fields. Consequently:

* the RNG state, revision and decision serial are restored exactly, so replaying
  the same action from the restored position produces the identical digest;
* the log is rebuilt from the prefix, so nothing the undone action revealed (a
  draw, a scry look, a private choice) is still readable anywhere;
* undo is refused after a sandbox override, because the recorded line no longer
  produces the live position — silently substituting a different game would be
  dishonest. Start a new game or use **Return to branch** instead;
* undo replays the recorded prefix synchronously inside the frame (CPU only, no
  IO, no engine rule evaluation beyond the recorded actions), so a very long
  recorded line costs the first frame after the key press;
* `Set branch / Return to branch` (issue23) keeps its own snapshot semantics and
  hidden-information boundary message.

A study branch also replaces the live position and clears the taint record, so
the branch itself has to prove the replacement is the clean recorded prefix. It
does: a sandbox override bumps the engine revision and decision serial, so a cut
that reaches into anything recorded *after* an override fails the replay with
`stale submission` and the tainted session is left exactly as it was; a cut
*before* the override replays the recorded prefix to the same digest the table
had at that point, which is what makes clearing the taint honest rather than
cosmetic. Both halves are pinned by
`a_sandbox_taint_cannot_be_branched_into_a_clean_line`.

## 5. Destructive actions ask first

New game (`N`), Start new table, Load (`L`), Branch from replay (`G`), Reset
*sandbox* game and Start imported match all set a pending confirmation that is
rendered in the persistent primary strip. The question names the action, the
seed or the file, so it is never a generic "are you sure?". Both branch triggers
(the Study panel button and `G`) go through one helper, so the shortcut cannot
skip the question the button asks. `Cancel` and `Esc` change nothing, and a
failure (load/branch/import/table start) is shown in the error colour, never
swallowed. Nothing destructive runs on the first click.

## 6. Evidence and limits

* `crates/core/tests/issue20_sandbox.rs` — deterministic controls, masked scry
  look, refusals, save/load taint, laundry refusal, zone/search projection.
* `crates/ui/tests/issue20_controls.rs` — undo restores digest/RNG/decision
  serial and the same continuation; undo refused on a tainted line; taint
  survives save/load; pending-choice and empty-library refusals; a post-override
  branch refused while a pre-override cut replays the clean digest.
* `crates/ui/src/app.rs` (viewport + input tests) — the inspector is searchable
  and the sandbox panel reachable at 1280×720 *and* at the 1024×640 programmatic
  minimum, the taint banner is in the fixed strip, a destructive action waits for
  its confirmation, `G` asks the same branch question the panel button asks, and
  a failed table start / imported start is an error state rather than a hint.
* `evidence/issue20/DELIVERY.md` — the packaged-app journey actually walked.

Not claimed: arbitrary 100-card Commander or partners; counters as rules objects
(they are storage/display only); a model playing strength; release timing; a
legal match result for any sandbox game.
