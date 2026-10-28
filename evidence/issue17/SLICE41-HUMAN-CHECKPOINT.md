# Issue17 — slice41 checkpoint

- Branch `aperture/issue-17`, base `a9e3429`; local head in final receipt. No push/merge/
  closure. Full #17 unfinished, 1 of 8 certified.

Implemented The Earth King (TLA 172): ETB distinct 4/4 green Bear token (CR 111.4 name,
nonimportable); attack trigger fires once for one or more controller's live 4+ power
attackers, captures the count, and runs an incremental private basic-land search (zero or
up to count, only basics, tapped canonical land entry, shuffle via engine RNG). Search is
owner-only in projection. Legend rule is not implemented; follows existing ordinary-legend
admission convention. Counts: 153 catalog / 141 supported / 318 missing; Ayula 12.

Gates: issue17_slice41 4 passed; printings 15, catalog_pool 5, slice15 2, commander 43,
replay 6; fmt/clippy/release receipts in `slice41/gates.log`. Journey: 4 seats/40 life,
terminal P3, 1780 decisions, 40 turns, Earth King cast 11, Bear tokens 11, tapped land
entries 18, shuffles 14; digest `d646deed…f127`, snapshot SHA256
`d7805f3f…0f58`, replay and save parity. No 100-card certificate.
