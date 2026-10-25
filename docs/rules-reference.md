# Rules provenance and conformance (practice slice)

This document pins the Magic: The Gathering Comprehensive Rules revision this
engine was written against, defines the executable conformance-case format used
to keep rule behavior honest, and states exactly which behaviors are the official
rule and which are deliberate, disclosed simplifications of the bounded practice
slice described in [ROADMAP.md](ROADMAP.md).

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

### Bounded card/source addition (branch candidate)

The branch candidate implements one additional real card, **Crack the Earth**,
bringing the candidate runtime pool from 15 to 16. It adds no other card,
ability, trigger, Commander profile or native multiplayer surface, and the two
bundled practice decks are unchanged.

| Field | Value |
| --- | --- |
| Official card page | <https://gatherer.wizards.com/BOK/en-us/98/crack-the-earth> |
| Captured source | `run35/crack-the-earth-official.txt` in the run scratch directory (not bundled in the repository) |
| Captured-file SHA-256 | `fa92b2dd888e51aa7a5bdcad7fa99fc1c70c3d8648d9e7f6375aceec5afd6777` (the capture header also records `06ffaf75f6131eb22fd0711e2da1fe6f060ad0a16b87d8d880944ce12757e1af` for the fetched body) |
| Name / cost / type | Crack the Earth; `{R}`; Sorcery — Arcane |
| Oracle text | “Each player sacrifices a permanent of their choice.” |
| Art | Not bundled; no card art is added to the repository |

### Command-zone practice source additions

The explicit command-zone practice profile adds five sourced cards (captured
2026-10-22 from official Gatherer pages by the independent Rules assignment;
Oracle text and costs are recorded in `Rules-1-result.json`):

| Card | Cost | Oracle |
| --- | --- | --- |
| Isamaru, Hound of Konda | `{W}` | vanilla 2/2 Legendary Creature — Dog; the only supported commander |
| Plains | — | Basic Land — Plains |
| Unsummon | `{U}` | Return target creature to its owner's hand. |
| Time Ebb | `{2}{U}` | Put target creature on top of its owner's library. |
| Swords to Plowshares | `{W}` | Exile target creature. Its controller gains life equal to its power. |

### Prevention source addition (branch candidate)

The branch candidate adds one full-combat-prevention instant, bringing the
runtime pool to 22. It is not a partial-prevention or redirection implementation.

| Field | Value |
| --- | --- |
| Official card page | <https://gatherer.wizards.com/J21/en-us/46/fog> |
| Captured source | `continuity/m2-commander-damage/evidence/kernel/fog-j21-46-gatherer.html` |
| Captured-file SHA-256 | `9e4af7c994548182e49851bf5ef162c0dd7888020ea8c5a03652417a4419c4fd` |
| Name / cost / type | Fog; `{G}`; Instant |
| Oracle text | “Prevent all combat damage that would be dealt this turn.” |
| Art | Not bundled |

The profile is **not** a Commander format implementation. It keeps 20 life and
one Isamaru per seat, refuses Isamaru in ordinary starting libraries (no legend
rule), and does not claim 903.4 color identity, 903.5 deck construction, 903.10
commander damage, 40 life, partners, meld/merge, or multi-commander variants.

The Arcane subtype is descriptive only in this slice; splice and other Arcane
interactions are unsupported. The candidate's sacrifice pipeline is a single
fixed-set public effect: it does not certify generic hidden-zone choices,
restarting/dependent APNAP choices, or any other CR 101.4 producer.

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

## 3. Official rule vs. current bounded scope

Anything not listed as implemented is unimplemented, not silently approximated.
The current branch candidate has 2–4 ordered headless seats, first-turn draw
policy, departure handling, masking, persistence replay v6/save v10, the harness,
ordered mulligan rounds and one fixed-set sacrifice/APNAP producer. Rows marked
**candidate** describe this branch, not an accepted M1 release. Independent
candidate review and exact-head checks are delivery gates; passing them does
not certify unsupported formats or complete rule coverage.

| Behavior | Official rule | Current bounded status | Where it is stated/tested |
| --- | --- | --- | --- |
| Multiplayer seats | CR 800, 806–810 | Ordered 2–4 seats in the headless/CLI candidate; the native table remains two-seat practice. Turn order skips eliminated seats without renumbering. | `m1_multiplayer.rs`, `HANDOFF.md` |
| Starting-player draw skip | CR 103.8a (two-player) and 103.8c/800.7 (other multiplayer) | Keyed to the **configured** game size: the two-player starting player skips the first draw; 3/4-seat games do not. A game that begins with more than two seats does not switch policy after an elimination. | engine comments, `m1_multiplayer.rs` |
| London mulligan | CR 103.5, 103.5c | **Branch candidate:** declarations start with the starting player and rotate in turn order; all eligible seats declare before any redraw; each mulligan redraws then bottoms before the next declaration round; kept hands are locked; the first 3/4-seat mulligan is free; limits are 8 for 3/4 seats and 7 for two seats. Bottom collection remains sequential after the batch, so simultaneous execution is not claimed. | candidate `mulligan` tests; `MULLIGAN_RULE_CORRECTION.md` |
| Fixed-set public sacrifice / bounded APNAP | CR 101.4, 101.4b, 608.2d, 701.21a | **Branch candidate:** only Crack the Earth produces this choice. The order is the active player then turn order, skipping departed seats; each live seat chooses one controlled permanent (or the engine-forced `None` when it controls none). All choices are collected before any permanent leaves; then valid chosen permanents move to their owners' graveyards as one deterministic batch. Sacrifice is not destroy or targeting, and no SBA/priority interleaves. This is not generic CR 101.4 support. | candidate APNAP/sacrifice tests; `APNAP_CONTRACT.md` |
| Hidden-zone/dependent APNAP choices | CR 101.4a, 101.4d | Unsupported and not claimed: the only candidate producer has public, fixed-set choices and cannot create a dependent/restarting choice. | this document; `APNAP_CONTRACT.md` |
| Combat damage division among blockers | CR 510.1c | Automatic lethal-first, one defending seat per declaration, no manual assignment or per-attacker defender split. | `multi-blocker-damage.json` `bounded_policy`, `combat_teaching.rs` |
| Cleanup-step priority | CR 514.3a | Cleanup never grants priority. | `HANDOFF.md` |
| Object identity across zones | CR 400.7 | Identity is stable. | `HANDOFF.md` |
| Counters | CR 122 | Stored/displayed only; no rules math. | `state.rs` comment |
| Fog full combat prevention | CR 614.1a, 615.1, 615.6 | **Branch candidate:** Fog sets one turn-scoped shield; every combat-damage instance to players and permanents is prevented before life/marked damage/commander-ledger credit, with a typed `CombatDamagePrevented` event. Cleanup clears the shield. Partial prevention/redirection and generic replacement effects are not implemented. | `catalog.rs`, `issue13_commander_combat.rs` |
| Activated abilities | CR 602, 602.2, 602.2b, 602.5a/b, 302.6 | **Branch candidate:** typed records in `ability.rs` for six catalog cards. Activation costs are `{T}`, mana, or "sacrifice this permanent"; one ability carries "activate only once each turn"; targets and modes are declared by the record, never by the caller. A `{T}` ability on a creature obeys summoning sickness. | `abilities.rs`, `ability.rs` |
| Triggered abilities | CR 603.2, 603.3, 603.3b/d, 603.5, 603.6a/c, 700.4 | **Branch candidate:** enters-the-battlefield and "dies" triggers, mandatory and optional ("you may", chosen on resolution). Simultaneous triggers are put on the stack in APNAP order from the active player; targets are chosen as the trigger is put on the stack and a trigger with no legal target is removed. Same-controller ordering is arrival order, not a CR 603.3b choice. | `abilities.rs`, `abilities_game.rs` |
| Last known information for abilities | CR 608.2b, 700.4 | **Branch candidate:** a "dies" trigger resolves from its recorded card and controller after the source has left the battlefield; a target that became illegal before resolution makes the ability fizzle. Full LKI for copied/merged objects and continuous effects is not implemented. | `abilities.rs` |
| Static abilities, replacement/partial prevention, tokens, layers | CR 604, 614, 111, 613 | Not implemented (Fog is the only prevention effect). | `catalog.rs`, `HANDOFF.md` |
| Combat damage to a departed player | CR 800.4e | Not assigned; the current candidate preserves this on departure. | `m1_multiplayer.rs` |
| Departed active player / turn completion | CR 800.4a, 800.4j, 800.4k | Implemented in the 2–4-seat candidate: the turn continues without an active player, priority passes to the next live seat, and a departed player's would-be turn does not begin. | `m1_multiplayer.rs`, `HANDOFF.md` |
| Simultaneous loss of all remaining players | CR 104.4a (with 104.4c, 104.5) | Engine draw projection; unexercised by the current pool. | `HANDOFF.md` |

### Persistence candidate

This branch writes replay v6/save v10 so the mulligan round, sacrifice queue,
serialize exactly. The previous published candidate (`74df9a6`) wrote replay
v2/save v3; those and older files are explicitly rejected, never reinterpreted.
The pinned `RULES_REVISION` remains `2026-09-25`; schema versioning is separate
from rules provenance. The branch remains a candidate subject to independent
review and exact-head verification, not an accepted M1 release.

### Unsupported formats and disclosures

Two-Headed Giant, shared team turns, non-London mulligan variants, Commander,
card art, native 3/4-seat tables and Arcane/splice interactions are out of scope.
Generic hidden-zone APNAP and dynamically restarting APNAP are not implemented,
and the single Crack the Earth producer must not be presented as full CR 101.4
coverage. The existing eight conformance cases remain two-seat evidence and are
retained unchanged; they do not certify the new card, mulligan round or
sacrifice pipeline.

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
