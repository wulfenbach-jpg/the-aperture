# Issue17 — slice21 human checkpoint

- Branch `aperture/issue-17`, base `cdfc32e`; local slice21 commit is the branch head in the
  final receipt. No push/merge/closure/PR creation.
- **Full #17 unfinished; 1 of 8 certified.**

## Implemented
**Dragon-Scarred Bear** `{2}{G}` 3/2 Bear, exact DTK 183 source
(`fixtures/starter-corpus/slice21-sourced.json`, retrieved `2026-10-27T04:44:50Z`).
Formidable `{1}{G}` repeatable activation; at activation only, total CURRENT power of the
activator's creatures must be >= 8 (layered counters/anthems, tokens, source included;
opponents/noncreatures excluded). Below threshold is not offered and a direct submission is
refused without state change.

Regeneration shields are typed, object-anchored, non-tapping, repeatable, and serialized
with `skip_serializing_if` empty so prior digests are unchanged. They replace only
destruction: lethal-damage SBA or the destroy-all effect. Replacement removes all marked
damage, taps, and removes the permanent from combat; multiple shields are consumed one per
destruction. Exile, bounce, sacrifice, and zero-toughness moves do not consume/apply a
shield. Unused shields expire at cleanup and cannot transfer to a new object id after a zone
change. Lethal damage SBA and the existing mass-destroy path share the replacement helper.

## RED retained
`cargo test -p aperture-core --test issue17_slice21` failed
`dragon_scarred_bear_is_sourced_and_supported` before implementation.

## Focused green
- issue17_slice21: 8 passed (offered cast/activation + Formidable refusal; lethal consumes
  shield, taps/clears, second hit dies; multiple shields; exile/bounce/new object; blocked
  combat regeneration removes attacker without player damage; save/reload live shields and
  identical final state; exact threshold 7→8 with counter and threshold fall after
  activation; zero-toughness and Crack the Earth sacrifice are not regeneration).
- Unit: cleanup-expiry test in engine.
- card_printings 8, catalog_pool 5, slice15 corpus 2, issue13 commander 43, replay_digest 6.
- fmt check exit 0; workspace clippy `-D warnings` exit 0; release CLI exit 0.
  Command-owned exits are persisted in `evidence/issue17/slice21/gates.log`.
- Legacy slice20 replay: `target/release/aperture-cli replay --file
  evidence/issue17/slice20/demo-replay.json` exit 0, digest
  `72cd69fb1143` matched (`evidence/issue17/slice21/legacy-slice20-replay.log`).
- Fresh audit: 138 catalog / 459 union = 126 supported + 333 missing; Ayula 27; snapshots
  unchanged.

## Journey
Evidence-only masked demonstrator, NOT product AI. Exact 4 seats / 40 life / seed 1919,
40-card `decks/package-slice21.txt`, Jasmine Boreal injected each command zone. Terminal
Win P3, 1906 decisions, 45 turns, snapshot SHA256
`f48f7a9011ac005858ce26b71e9313751694d0773797c815c5158480fb880003`; Dragon-Scarred Bear
cast 7, ability activated 25 times (derived from `Event::AbilityActivated`), exactly 25
shield-creation events, 1 actual regeneration;
replay matches and fresh save round-trips. Digest
`0d20c7597183ecb764f7b858dd87ca9aa654e3a4ac6984bada01e9df27926520`.
Not a 100-card starter certificate.

## Deviations / disclosures
- Formidable is a bounded card-specific check at activation; ability-word/restriction rules
  are 207.2c and 602.1b/602.5 (not a generic keyword framework).
- AI delta is mechanical initialization only: `crates/ai/src/search.rs:727` local GameState
  construction and `crates/ai/tests/common.rs:83` fixture gained the new shield field; no
  policy or search behavior change. The journey driver reads only public battlefield/shield
  bookkeeping for evidence instrumentation, never hidden libraries or opponent hands, and
  submits only projected legal actions.
- No full workspace matrix rerun; exact-head CI owns it.
