# Issue 23 — live-session persistence, replay branches and postgame review

Bounded practice slice. 2–4 live seats, 28-card supported pool, 40-card bundled
lists, one supported commander per seat for the bounded 40-life profile. Not
Commander, not arbitrary 100-card lists.

## What the app does now

- **Save (S) / load (L)** write and restore `SessionSave` version 10. The save
  carries the full engine state (pending choice, stack, RNG position, decision
  serial) plus the embedded action line; loading is verified by re-running that
  line on the load worker and comparing the canonical digest.
- **Study (B) → G / “Branch from replay”** takes a session save (its embedded
  replay) or a bare replay file and an action index. The branch keeps exactly
  that recorded prefix, replays it under the same seed on a worker, and
  applies the result only if its generation still matches. Every later accepted
  action is branch history; the base tail after the branch point is never read
  into the live game.
- **Branch comparison** lists the first recorded index where the branch and the
  base stop agreeing, using only `(seat, decision serial, action kind)` — no card
  names, targets or zone contents — so the default comparison leaks no private
  information. The compared span never reaches past the branch's own recorded
  history: on a live branch the base continuation after the divergence stays
  unrevealed, marked only by the note “its continuation is not revealed here”.
- **Postgame review (R)** is offered only once `outcome` is set. While it is open
  the primary strip is replaced by the omniscient readout (every seat’s hand,
  library, graveyard, exile and command zone), every submission and sandbox tool
  is refused, and `tick_ai` is suspended: the review is never a policy input.
- **Generation guard**: `invalidate_pending_work()` bumps a monotonic generation
  on new game, load, branch, imported match start, sandbox edit, AI opt-in and
  any accepted action. Loads and branches captured before the bump are
  discarded with a “discarded a stale …” note; AI, import and health results
  additionally re-check revision, decision serial and seat.

## CLI

`aperture-cli replay --file <save-or-replay>` verifies as before. New:

- `--branch-at N [--branch-out out.json]` cuts/verifies a branch at recorded
  action N (replays the prefix deterministically).
- `--compare-with other.json` prints the public accepted-action comparison.
- `--save-out out.json` writes a certified session save of a finished replay’s
  final state (refuses replays with no recorded outcome).

## Evidence

`evidence/issue23/EVIDENCE.md` records the focused tests, the packaged-app
journey and its AX text captures, and the CLI outputs for the finished 4-seat
game (`finished.replay.json`, digest `c76c56bd72fd`, 1104 actions, P0 win on
CR 704.5a).

## Disclosed limitations

- Bounded 28-card practice, one supported commander per seat; no partners, no
  legend rule, no arbitrary Commander support, no measured AI playing strength.
- Branch comparison is deliberately kind-level only; it is a study aid, not a
  hidden-information oracle.
- Unsigned Windows build.
