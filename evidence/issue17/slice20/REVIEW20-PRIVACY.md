# Slice 20 privacy repair — re-adjudication

**APPROVED_INCREMENT** at `41d55ee87bbd1de200b40df92ea06d80aecf9516` (base `78dea97`).

The prior APPROVED_INCREMENT at `f4a02a2` is **superseded** by the independently
proven privacy leak; this is a new verdict for the repair, not a silent
retroactive approval of the old head.

## Verified

- Product diff `78dea97..HEAD` is exactly three lines in `crates/core/src/view.rs`
  (owner-only `Pending::TopCardChoice` filtering), the permanent
  `issue17_slice20_privacy.rs` regression, and the privacy-repair receipt. No
  engine/profile/AI changes.
- `cargo test -p aperture-core --test issue17_slice20_privacy --test issue17_slice20 --test projection --locked`:
  privacy **1/1**, slice20 **9/9**, projection **4/4**.
- The privacy regression compares the full opposing `PlayerView` for identical
  public state with an eligible Grizzly Bears vs ineligible Forest hidden on top,
  including the masked digest, and passes. Owner still receives the identity,
  `pending_viewed`, and take/bottom offers; opponents receive a neutral pending
  shape and no viewed card.
- `replay --file evidence/issue17/slice20/demo-replay.json`: unchanged 2064
  actions / 46 turns / 5281 events, digest `72cd69fb1143…` matches.
- `cargo fmt --all -- --check` and `git diff --check` pass.

No blocking finding remains. Full original #17 is still unfinished: **1/8
certified**, 137 catalog / 459 union = 125 supported + 334 missing, Ayula 28
missing. No merge, closure, or downstream unlock. Exact-head CI remains
parent-owned.
