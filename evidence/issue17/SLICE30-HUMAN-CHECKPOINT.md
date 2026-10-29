# Issue17 — slice30 checkpoint

- Branch `aperture/issue-17`, base `de3ba07`; local head in final receipt. No push/merge/
  closure/PR creation. Full #17 **unfinished, 1 of 8 certified**.

Drover Grizzly implemented with a reusable typed saddle action, sorcery timing, tapped
effective-power helper cost, bounded offered subsets, stack resolution, until-end saddled
designation, typed attack condition, and layer-6 trample grants. Counts refreshed to
146 catalog / 134 supported / 325 missing; Ayula 19. Focused issue17_slice30 4 passed;
printings/catalog/pins/replay green; fmt/clippy/release exit 0. Journey terminal P1,
2653 decisions, 55 turns, Drover cast 9, saddle activations 116, trample grants 22,
digest `b711d966…5677`, replay and save parity.

Repair: all new saddle/grant rules use live `Game::is_creature` (animated Stuffed Bear helper
and recipient tested); saddle stack `affected` binding is invalidated on source departure;
cleanup/source-left controls and actual multi-helper offers added. Bounded offered subsets
cap at 10 eligible helpers; above the cap no offers are generated while direct validation
remains general. This is a disclosed limitation, not full-card conformance. No same-ID legal
reentry control beyond no-effect source-left. No full CI locally; exact-head CI owns it.
