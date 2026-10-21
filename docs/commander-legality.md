# Commander deck-profile legality (offline analysis only)

Status: **M2 issue #11 implementation; independent review and native-owner
confirmation pending.** This is a deck-profile legality analyzer, not playable
Commander. It never creates a practice `DeckList`, never calls `plan_deck`, and
never starts a game.

Pinned rules: Comprehensive Rules effective **2026-09-25** (the same
`RULES_REVISION` as the engine). Source capture hashes are recorded in the
phase's `rules-interface.md` and the fixture manifest.

## Command

```text
aperture-cli commander-check --deck <deck-snapshot.json> --cards <cards-snapshot.json>
                             [--json] [--out <report.json>]
```

Exit codes: `0` legal, `1` illegal, `2` not_certified **or** an
input/schema/provenance error. `not_certified` never exits 0. The command is
fully offline and deterministic.

The native/CLI practice file loader is deliberately separate. It accepts card
lines and the acknowledged `Deck` or `Mainboard` grouping header only. Every
other non-card section header is refused by `DeckList::parse_practice` instead of being
flattened into a 40-card practice list, including Commander-family names and
count-suffixed forms such as `Commander (1):` and `Sideboard (15):`. Valid
40-card practice files are unchanged.

## Input snapshots

### `aperture.commander-deck.v1`

```json
{
  "schema": "aperture.commander-deck.v1",
  "provenance": {
    "kind": "real | constructed | synthetic",
    "provider": "offline snapshot provider",
    "source": "where the deck bytes came from",
    "captured_at": "RFC 3339",
    "raw_sha256": "64 hex chars over the exact raw_deck UTF-8 bytes"
  },
  "raw_deck": "{\"commanders\":[...],\"library\":[...],\"companions\":[],\"sideboard\":[],\"excluded\":[]}"
}
```

`raw_deck` is the exact captured/constructed zone document; the typed zones are
parsed from it. `commanders` and `library` are distinct zones. Each entry needs
a non-empty English `name`, a non-empty `oracle_id`, and a quantity in
`1..=255`. A commander quantity other than 1 is an `illegal` finding.

### `aperture.card-metadata.v1`

```json
{
  "schema": "aperture.card-metadata.v1",
  "provenance": {
    "kind": "real | synthetic",
    "provider": "scryfall | synthetic",
    "api_url": "https://api.scryfall.com/bulk-data",
    "download_url": "https://data.scryfall.io/oracle-cards/...jsonl.gz",
    "bulk_type": "oracle_cards",
    "api_updated_at": "RFC 3339",
    "captured_at": "RFC 3339",
    "cards_sha256": "64 hex chars over the exact raw_cards_jsonl UTF-8 bytes",
    "raw_sha256": "64 hex chars over the SAME exact raw_cards_jsonl bytes",
    "selection": "how the subset was chosen",
    "card_count": 39
  },
  "raw_cards_jsonl": "<selected JSONL lines, exactly as captured>"
}
```

**Trust boundary:** both `cards_sha256` and `raw_sha256` bind the exact embedded
`raw_cards_jsonl` UTF-8 bytes that were analyzed. A detached, zero or unrelated
`raw_sha256` is rejected; no full-bulk digest is verified or presented because
those bytes are not embedded. The digests prove integrity of the embedded bytes,
not that a remote server actually served them. The report discloses the metadata
capture time, age in days, and staleness (>30 days); it never claims a live
ban-list lookup.

Real wrappers must use `provider: "scryfall"`, an `api_url` on a Scryfall host,
and `download_url` on `data.scryfall.io`. Synthetic fixtures must use
`provider: "synthetic"` plus a source label; the report's provenance records
which kind was used.

Fixture files under `crates/import/tests/fixtures/commander/real/` are a small
**real captured Scryfall subset** with `manifest.json` (source URL, capture
time, per-line digests, bulk digest, Wizards/Scryfall licensing note). The
legal-100 deck is explicitly marked `constructed` from those identities. Test
mutations are generated at runtime and marked adversarial/synthetic.

## Result

`aperture.commander-report.v1` carries:

* `status`: `legal | illegal | not_certified` for the deck profile.
* `deck`: commander/library/total counts and unique canonical names.
* `commanders`: separate commander entries with color identity, layout,
  eligibility status and reason.
* `format_legality`: recorded-snapshot ban status, `as_of`, `age_days`,
  `stale`, `snapshot_sha256`, and an explicit "not live" basis.
* `engine_support`: `supported | unsupported` with concrete reasons. This is
  independent of format legality.
* `gameplay`: always `available: false`, with command zone/return, commander
  tax, 40 life, 21 commander damage and multiplayer/seat reasons.
* `findings`: per-rule `legal | illegal | not_certified` records.
* `card_evidence`: top-level color-identity union and per-face evidence for
  non-normal cards, so a refusal is auditable.
* `provenance`: deck and card snapshot provenance.

Status precedence: a proven violation is `illegal`; otherwise any
missing/ambiguous/unsupported rule is `not_certified`; otherwise `legal`.
Missing metadata can never produce `legal` and cannot by itself produce
`illegal`.

## Implemented rules

* **CR 903.5a:** exactly 100 cards including the commander (99 and 101 are
  illegal).
* **CR 903.5b:** singleton by canonical English name. The exemption requires the
  actual **Basic Land** supertype/type before the type separator, not a subtype
  or rules-text word. Duplicate cards with a card-specific allowance
  ("a deck can have any number / up to N cards named ...") are
  `not_certified`, never falsely illegal. Interchangeable-name/unknown static
  deck-construction text is `not_certified`.
* **CR 903.3:** baseline commander is a front-face **legendary creature**.
  Vehicles and Spacecraft are CR-legal commander categories but this slice does
  not model their power/toughness/zone semantics; they are detected from the
  type-line subtype segment and refused as `not_certified`, never falsely
  illegal. The same applies to zone-dependent/type-altering off-battlefield
  eligibility such as Grist, the Hunger Tide (a legendary planeswalker that is
  a creature while not on the battlefield). Non-creature "can be your
  commander" cards, all non-normal layouts, chosen-color-before-the-game
  commanders (CR 903.4b, e.g. Faceless One), and Partner/Background/Friends
  forever/Doctor's companion variants are also explicitly `not_certified`.
  Variant detection cross-checks Scryfall `keywords` **and** Oracle rules text,
  so a partner card whose keyword array is empty is still refused. A deck entry
  using a `printed_name` alias that does not match the canonical Scryfall name
  for the same oracle id is an ambiguous identity (`not_certified`); alias
  names are never used to certify identity.
  **Every 2+ commander deck is `not_certified`** (conservative director
  decision).
* **CR 903.4:** color identity uses the authoritative top-level Scryfall
  `color_identity` (including back-face union). There is **no front-only
  certification**: every non-normal layout is `not_certified`, and face evidence
  is retained. Reminder text is not separately reconstructed.
* **CR 903.5c:** a library card whose identity is outside the single
  commander's identity is illegal.
* **CR 903.5d:** basic land types are an additional restriction. A card with a
  basic land type that could produce an off-identity color is illegal even when
  its recorded `color_identity` is empty (for example a synthetic Forest with
  empty identity under a colorless commander). `Wastes` has no basic land type
  and is not restricted by 903.5d.
* **CR 903.5e:** a non-empty sideboard is `not_certified` and is never counted.
  Companion entries are `not_certified` and never counted as a 101st card.
* **Bans:** recorded `legalities.commander`; `banned`/`not_legal` are illegal,
  `restricted`/missing/unknown is `not_certified`.
* **Identity/provenance:** duplicates sharing an oracle id must agree on
  relevant metadata (name, layout, type line, color identity, legality, rules
  text including nullness, keywords, produced mana, faces). Conflicts are
  `not_certified` in either array order; only equivalent printings are
  tolerated. A null `oracle_text` is incomplete semantics (`not_certified`),
  not an empty rules text. Unknown type-line supertype/type tokens are
  `not_certified`, not parsed as a legendary creature.
* **No laundering:** a Commander report or deck snapshot does not deserialize as
  the practice `DeckList`/`DeckReport`, and the practice launch path refuses
  Commander text. There is no substitution path.

## Explicitly not implemented

Playable Commander (command zone/return, tax, 40 life, 21 commander damage,
multiplayer seats), partner/background certification, a broad face-aware rules
engine, Archidekt raw-deck adaptation, live ban lookups, and engine support for
the analyzed cards. A legal format report never means the deck can be launched.
