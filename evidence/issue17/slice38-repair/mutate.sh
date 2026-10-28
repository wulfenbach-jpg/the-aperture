#!/usr/bin/env bash
# Mutation receipt for review 301's blocking defect (slice 38 repair).
#
# The defect was a registration gap: CardId::FoodToken existed as a creatable
# token card but was absent from CardId::is_token()/TOKEN_CARDS, so every CR 111.7
# path treated the sacrificed Food as an ordinary card. This runner removes the
# variant from `is_token()` — the single line the repair added — re-runs the lane
# suite, restores the file from a saved copy and proves the restored bytes with
# `sha256sum -c`. The RED log must show the three discrimination tests failing.
#
# Hashes bind the CHECKED-OUT file (per the review-294/301 note); on a CRLF
# export the file hash differs from the git blob hash even though the bytes are
# the same source.
set -u
cd "$(dirname "$0")/../../.." || exit 1
OUT=evidence/issue17/slice38-repair
CARD=crates/core/src/card.rs

sha256sum "$CARD" > "$OUT/pre-mutation-card-rs.sha256" || exit 1
cp "$CARD" "$OUT/card.rs.bak" || exit 2

# Remove exactly the added match arm, leaving the TOKEN_CARDS registry entry.
perl -0pi -e 's/\n                \| CardId::FoodToken\n(        \)\n    \})/\n$1/' "$CARD" || exit 3
echo "== mutated is_token() ==" | tee "$OUT/mutation-is-token-red.log"
grep -n "FoodToken" "$CARD" | tee -a "$OUT/mutation-is-token-red.log"

cargo test -p aperture-core --test issue17_slice38 --locked \
  >> "$OUT/mutation-is-token-red.log" 2>&1
echo "mutation_test_exit=$?" | tee -a "$OUT/mutation-is-token-red.log"

cp "$OUT/card.rs.bak" "$CARD"
rm -f "$OUT/card.rs.bak"
{
  echo "== restored =="
  sha256sum -c "$OUT/pre-mutation-card-rs.sha256"
} | tee "$OUT/mutation-restored.log"
