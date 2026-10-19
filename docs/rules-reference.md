# Rules provenance and conformance (practice slice)

This document pins the Magic: The Gathering Comprehensive Rules revision this
engine was written against, defines the executable conformance-case format used
to keep rule behavior honest, and states exactly which behaviors are the official
rule and which are deliberate, disclosed simplifications of the bounded
two-seat practice slice described in [ROADMAP.md](ROADMAP.md).

It is not a copy of the Comprehensive Rules. The complete third-party document
is deliberately **not** bundled in this repository; only its provenance and
short cited references are recorded here.

## 1. Pinned official revision

| Field | Value |
| --- | --- |
| Document | Magic: The Gathering Comprehensive Rules (`MagicCompRules 20260925`) |
| Effective date | **2026-09-25** |
| Primary landing page | <https://magic.wizards.com/en/rules> |
| Pinned artifact (plain text) | <https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt> |
| Also published as | `MagicCompRules 20260925.pdf`, `MagicCompRules 20260925.docx` (same revision, different containers) |
| Retrieved | 2026-10-19 |
| Bytes / lines | 977752 bytes, 9372 lines |
| SHA-256 | `8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca` |

The engine constant `aperture_core::RULES_REVISION` is `2026-09-25`. The same
string is written into `GameState.rules_revision`, `Replay.rules_revision` and
each conformance case file, so evidence can never silently target a different
revision.

Reproduce the hash from a clean download (PowerShell example):

```text
Invoke-WebRequest 'https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt' -OutFile MagicCompRules-20260925.txt
(Get-FileHash MagicCompRules-20260925.txt -Algorithm SHA256).Hash
```

Re-downloading and hashing is the only provenance check; a search-engine snippet,
a mirror, or an assistant's memory is not an acceptable source.

## 2. Executable conformance cases

* Runner: `crates/core/tests/rules_conformance.rs`
* Cases: `crates/core/tests/fixtures/rules/*.json`
* Manifest: `crates/core/tests/fixtures/rules/manifest.json`
  (`schema`, `rules_revision`, `source_sha256` only; the human provenance in
  section 1 is not duplicated into the machine-readable manifest)

Run exactly this suite with:

```text
cargo test -p aperture-core --test rules_conformance --locked
```

The runner is data-driven. Each case states a symbolic initial position, the
actions a seat submits, the decision/events the engine must produce, and the
resulting state. Every case cites the rule numbers it exercises, and any case
that documents a divergence from the official rule must carry a
`bounded_policy` note. The runner rejects unknown JSON fields, so a typo fails
loudly instead of being ignored. Only the actions, events and state overlays
listed below are part of the format; an unsupported name is a hard failure, not
a silent skip.

### Schema `aperture.conformance/1`

```text
{
  "schema": "aperture.conformance/1",
  "rules_revision": "2026-09-25",
  "source_sha256": "<64 hex chars>",
  "cases": [
    {
      "id": "timing.example",
      "description": "One sentence: what is pinned.",
      "rule_refs": ["CR 117.1a", "CR 307.1"],
      "bounded_policy": "optional; required when the case documents a divergence",
      "initial": {
        "seed": 101,
        "turn": 1,
        "active": "P0",
        "step": "PrecombatMain",
        "priority": "P0",                // or "none"
        "life": {"P0": 20, "P1": 20},
        "zones": {
          "P0": {
            "hand": ["Forest"],
            "battlefield": ["Island", "Island", "Island"],
            "library": ["Forest", "Mountain"]
          }
        },
        "permanents": {
          "Grizzly Bears": {"damage": 2, "pump_power": 0, "pump_toughness": 0}
        }
      },
      "steps": [
        {
          "player": "P0",                // omitted: whoever owns the decision
          "action": {"type": "pass"},
          "expect": "accepted",          // or "rejected"
          "error": {                     // required when expect is "rejected"
            "type": "illegal",           // or "bad_timing"
            "action": "play land"        // "name" for bad_timing
          },
          "events": [                    // expected events, in order; extra events may fall between
            {"type": "Pass", "player": "P0"},
            {"type": "StepBegin", "step": "BeginCombat"}
          ],
          "expect_pending": "DeclareAttackers",  // "none" | Pending variant name
          "expect_priority": "P0",               // "none" for no priority holder
          "expect_step": "DeclareAttackers",
          "expect_turn": 1
        }
      ],
      "expect": {
        "life": {"P0": 20, "P1": 20},
        "turn": 1,
        "step": "PrecombatMain",
        "priority": "P0",
        "pending": "none",
        "zones": {
          "P0": {
            "hand": ["Forest"],              // unordered multiset
            "library": ["Island", "Forest"], // ordered: reordering is a bug
            "graveyard": ["Shock"],
            "battlefield": ["Grizzly Bears"]
          }
        },
        "battlefield_state": {"Grizzly Bears": {"damage": 0, "pump_power": 0}},
        "outcome": {"winner": "P1"}
      }
    }
  ]
}
```

Symbolic references make cases seat/object-id independent:

* Cards are named by their Oracle name (`"Grizzly Bears"`, `"Craw Wurm"`); the
  runner resolves them to `CardId`, then to the first unused matching object in
  the requested zone.
* Repeated names (for example two `Shock` discards) are consumed in order.
* `next_object` allocation order is therefore irrelevant to the expectations.
* Library assertions preserve order; hand, graveyard and battlefield assertions
  compare multisets. A shuffled library is a real bug that a multiset cannot see.
* Rejections match only the typed `EngineError` variant and its structured fields
  (`action` or `name`). The human-readable `reason()` text is deliberately not
  asserted: it is presentation, and pinning its wording would enshrine phrasing
  rather than the rule.

Supported actions: `pass`, `play_land`, `cast_spell`, `activate_mana_ability`,
`declare_attackers`, `declare_blockers`, `discard`. Supported event matchers:
`TurnBegin`, `StepBegin`, `Priority`, `Pass`, `LandPlayed`, `ManaTapped`,
`Cast`, `Resolve`, `Draw`, `Discarded`, `AttackersDeclared`,
`BlockersDeclared`, `CombatDamage`, `Destroyed`, `GameOver`. Every matcher
compares structured payload fields; nothing matches a description or message
substring. State-based actions are covered by observable state (a creature in
the graveyard, a game outcome), not by matching SBA narration.

### Case index

| File | Case id | Rules cited |
| --- | --- | --- |
| `timing-priority.json` | `timing.sorcery_is_rejected_outside_its_controllers_main_phase` | CR 117.1a, 307.1 |
| `timing-priority.json` | `timing.land_once_then_sorcery_resolves_in_main_phase` | CR 117.1a, 305.2, 307.1, 601.2a, 608.2 |
| `state-based-actions.json` | `sba.lethal_damage_sends_the_creature_to_the_graveyard` | CR 704.5g |
| `state-based-actions.json` | `sba.zero_toughness_sends_the_creature_to_the_graveyard` | CR 704.5f |
| `state-based-actions.json` | `sba.player_at_zero_life_loses_the_game` | CR 704.5a, 104.3c |
| `cleanup.json` | `cleanup.discard_down_to_maximum_hand_size` | CR 514.1, 402.2 |
| `cleanup.json` | `cleanup.removes_marked_damage_and_until_end_of_turn_effects` | CR 514.2, 500.5 |
| `multi-blocker-damage.json` | `combat.multi_blocked_attacker_splits_damage_lethal_first` (bounded policy) | CR 509.1a, 510.1c, 510.2, 704.5g |

These eight cases are two-seat practice-slice evidence. They are not a claim of
complete rule coverage; each milestone adds cases with the cards and mechanics
it actually certifies.

## 3. Official rule vs. bounded policy

Anything not listed as implemented is unimplemented, not silently approximated.

| Behavior | Official rule | This slice | Where it is stated/tested |
| --- | --- | --- | --- |
| Multiplayer seats | CR 800, 806–810 | Two seats only (M1 adds live seats separately) | `HANDOFF.md` §10, `RULES_REVISION` scope |
| Starting-player draw skip | CR 800.7: multiplayer starting player does **not** skip | Two-player skip is implemented (CR 103.8a); when multiplayer arrives it must be conditional on the configured game size (CR 100.1a/b, 800.1), not on surviving players | engine comment, `sba`/timing fixtures |
| London mulligan | CR 103.5; CR 800.6 first multiplayer mulligan is free | Two-player London mulligan | `replay_digest.rs` |
| Combat damage division among blockers | CR 510.1c: the attacking player chooses the division | Automatic lethal-first, no assignment choice | `multi-blocker-damage.json` `bounded_policy`, `combat_teaching.rs` |
| Cleanup-step priority | CR 514.3a may grant priority when SBAs or triggers apply | Cleanup never grants priority | `HANDOFF.md` §10 |
| Object identity across zones | CR 400.7: a new object is created per zone change | Identity is stable | `HANDOFF.md` §10 |
| Counters | CR 122 | Stored/displayed only; no rules math | `state.rs` comment |
| Triggers, activated abilities, statics, replacement/prevention, tokens, layers | CR 603, 602, 604, 614, 111, 613 | Not implemented (only basic-land mana abilities exist) | `catalog.rs`, `HANDOFF.md` §10 |
| APNAP simultaneous choices | CR 101.4 | No supported effect or pending choice creates a simultaneous multi-player choice; declared attackers/blockers, mulligan, bottom, discard and scry/surveil are single-seat decisions | this document; must not be claimed as covered |
| Combat damage to a departed player | CR 800.4e: not assigned | Not applicable while only two seats exist | handed to the M1 seat/elimination work |
| Departed active player / turn completion | CR 800.4a, 800.4j, 800.4k | Not applicable while only two seats exist | handed to the M1 seat/elimination work |

The first-player draw skip is the one two-player-only rule that becomes
*incorrect* (not merely incomplete) with more seats: CR 103.8c says that in all
other multiplayer games no player skips the draw step of their first turn
(restated by CR 800.7). That change must arrive with the multiplayer scheduler
and its own fixtures, not by editing the two-player expectation.

### M1 migration notes (planned, not yet implemented)

These are pinned citations for the multiplayer work, recorded here so the
two-seat evidence is never silently rewritten. None of them is implemented in
the current build, and none may be claimed as covered until its fixtures exist.

| Planned behavior | Official rule | Note |
| --- | --- | --- |
| 3–4 player games draw on turn one | CR 103.8c, 800.7; classification: 100.1b, 800.1 | Keyed to the **configured** game size (a game that *begins* with more than two players), not to the number of surviving players. This is fixed at game setup and never switches after an elimination; the two-player skip stays for a game configured for two. |
| First multiplayer mulligan is free | CR 103.5c, 800.6; classification: 100.1b, 800.1 | Also keyed to the **configured** game size (three or more players at game start), not to survivors. Eliminating a player never grants or removes a free first mulligan mid-game, and the free mulligan does not count toward bottoms or the mulligan limit. |
| Mulligan limit | CR 103.5 | The slice's limit must be re-checked against the multiplayer count, not assumed from the two-player value. |
| Mulligan declarations | CR 103.5 | Official play is simultaneous declaration then simultaneous mulligans; sequential prompts remain a disclosed bounded deviation. |
| Simultaneous loss of all remaining players | CR 104.4a (with 104.4c and 104.5) | Draws project to an engine draw outcome; unexercised by the current card pool. |
| APNAP ordered choices | CR 101.4 | No supported effect or pending choice creates a simultaneous multi-player choice, so this stays explicitly unsupported/unexercised. |
| Seat departure mid-turn | CR 800.4a, 800.4j, 800.4k | Turn continues without an active player; see the handoff for the internal removed-zone representation. |

Two-Headed Giant, shared team turns and non-London variants are out of scope and
must not be implemented or implied by these notes.

## 4. Reviewing a new rules revision

The goal is that old evidence and old replays are never silently reinterpreted
by a new document.

1. **Retrieve from the primary source.** Fetch the plain-text document from
   `magic.wizards.com/en/rules`, record the new effective date, byte/line count
   and SHA-256. Do not adopt a revision from a mirror, blog, or tool cache.
2. **Diff before deciding.** Compare old and new text for the sections this
   slice cites (this document's `rule_refs`). Classify each difference:
   not applicable to the supported pool; wording-only; behavioral.
3. **Wording-only or not applicable.** Update `RULES_REVISION`, the manifest and
   the fixture `rules_revision`/`source_sha256` fields in one reviewed commit,
   and rerun `cargo test -p aperture-core --test rules_conformance --locked`.
   The runner fails if the fixtures and the engine disagree about the revision.
4. **Behavioral change.** Do not edit an expected value to make the new engine
   pass. Add or update the affected fixture with the new `rule_refs` and a
   comment stating what changed and why, in the same commit as the engine change.
   Keep the old expectation recoverable in commit history, and note the change
   in the milestone handoff.
5. **Replays and saves.** A replay records the `rules_revision` it was produced
   under, and a save records the state shape version. A revision change must
   never rewrite an existing replay file, its `rules_revision` or its recorded
   `final_digest`. Re-verify an old replay against a build pinned to its recorded
   revision, or revalidate it explicitly and record the result; if the digest
   changes, that is a reported break, not a baseline refresh. Bump
   `REPLAY_VERSION`/`SAVE_VERSION` only for shape changes, separately from a
   rules revision.
6. **Keep the scope honest.** Update the bounded-policy table above and any
   public README/HANDOFF statement the revision invalidates. Unsupported
   mechanics stay unimplemented; they do not become "close enough".

## 5. Evidence

Raw runs are kept outside the repository (local scratch), not committed:

* Official document retained at `rules-source/MagicCompRules-20260925.txt` in the
  run scratch directory, with the SHA-256 above.
* GREEN: `cargo test -p aperture-core --test rules_conformance --locked` — 5
  test functions, 8 cases, exit 0 (`rules-evidence/conformance-green-final.log`).
* RED (negative controls): two independent assertions were deliberately
  perturbed, each producing exit 101 with the suite failing, then restored and
  re-run green:
  * `conformance-red-library-order.log` — final library order swapped in the
    Divination case; failure message is `P0 library (order matters)`, proving the
    ordered comparison detects reordering that a multiset would miss.
  * `conformance-red-combat-event.log` — one expected `CombatDamage` amount
    raised from 2 to 3 in the multi-blocker case; failure names the missing
    event after a real event index, proving event assertions are not vacuous.

Recorded paths and exit codes for the specific run are listed in the milestone
handoff; this document is the durable specification of the format and procedure.
