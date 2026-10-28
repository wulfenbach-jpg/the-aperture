# Issue17 — slice42 checkpoint

- Branch `aperture/issue-17`, base `728e776`; local slice42 head in final receipt. No push/
  merge/closure. Full #17 unfinished, 1 of 8 certified.

Implemented shared Equipment package Lightning Greaves and Swiftfoot Boots: ordinary artifact
Equipment cast path, typed attachment effects, printed equip activated abilities (0/1,
sorcery timing, own target), attachment timing on resolution, multiple/moving attachments,
off-field/incarnation cleanup, live haste and shroud/hexproof queries through central target
validation. Greaves shroud blocks all targeting; Boots hexproof blocks opponents only;
player targets/nontarget effects unaffected. Counts: 155 catalog / 143 supported / 316 missing;
Ayula 10 missing.

RED: `issue17_slice42 equipment_sources_are_registered_and_castable` failed before card
registration. Focused: issue17_slice42 5 passed; card_printings 15, catalog_pool 5, slice15 2,
commander 43, replay 6; fmt/clippy/release exit 0 (`slice42/gates.log`). Journey: 4 seats/40
life, terminal P2, 1565 decisions, 36 turns, both equipment cast, 17 equip activations, 48
combat-damage events; digest `1750a81e…f551`, snapshot SHA256 `05cb383d…6dcb`, replay and
save parity. Native animation defect and minimal fix retained: recompute_layers must query
creature type before removing animation effects. Added same-ID target recast, old-attachment
survival on illegal new target, and real Prodigal Sorcerer haste tap-offer controls. Journey
driver now uses projected turn number and reports 17 successful attachments; header corrected.
Residual gaps: target-protection-at-resolution and explicit wrong-phase/not-owner/cost refusal
controls are incomplete; no same-turn equipped haste-attack event proof. No 100-card certificate.
