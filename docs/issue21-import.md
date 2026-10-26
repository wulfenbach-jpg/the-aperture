# Issue21 — Archidekt import is a complete, honest start flow

This document describes what the import lane actually delivers after this change, and
what it deliberately refuses to claim. It is written for the person reading the app, not
for the model that produced it.

## What the flow does

1. **A source per seat.** Each of the 2–4 seats accepts a canonical public Archidekt URL
   (`https://archidekt.com/decks/<digits>[/slug]`), a pasted text list, or a deck file.
   The URL is never fetched verbatim: the request is rebuilt from the numeric deck id
   against the allowlisted host (`archidekt.com`, `www.archidekt.com`), redirects are
   followed at most once and only to an allowlisted host, the request has a 15 s timeout
   and the body is read through a hard 8 MiB cap.
2. **Certification on two separate axes** (`aperture_import::start::certify`):
   * **format legality** — Commander construction from the provider's own card data: one
     commander, 100 cards including it, singleton (basics unlimited), color identity
     inside the commander's identity. A check the payload cannot answer is reported as
     `not_certified`, never assumed legal. `not_certified` and `illegal` are different
     answers and are shown as different answers. The singleton check looks at every card
     in the library, including cards this build cannot seat: engine support never narrows
     a format check. Basics are exempt only on a fact (the provider's `Basic` supertype,
     or the catalog's basic-land data); a repeat whose source carries no type line at all
     is `not_certified`, not `legal`.
   * **engine support** — every main-deck card against this build's supported pool,
     with the exact reason per card (not in the pool, multi-face, commander-only, no
     provider card data, …). Counts and the full list are shown before start.
3. **Seat fit** — a third, plainly separate question: does the selected profile seat this
   list? Ordinary practice needs exactly 40 main-deck cards and no command zone; the
   bounded command-zone profiles need a certified commander and at least 40 library
   cards, and they play the deck's own library exactly as listed.
4. **Apply and start.** A seat becomes startable only when engine support and seat fit
   both pass. The exact list is applied verbatim and the imported commander (when the
   profile has a command zone) replaces the bundled one for that seat. A refused deck
   keeps the live table untouched.

Nothing in the flow substitutes, trims, approximates or re-bundles a deck. There is no
fallback to a bundled list: a refused import stays refused and says why.

## What it does not claim

* **Format legality is not engine support.** A deck can be Commander-legal and completely
  unplayable here (the supported pool is 31 cards), and a list can be seatable without
  being Commander-legal. Both verdicts are shown; neither is inferred from the other.
* **A successful import is not a playable Commander deck.** The bounded profiles are not
  the Commander format: no 100-card construction requirement, no partners, no legend
  rule, no arbitrary card support. The 100-card *format* rule is reported, not enforced.
* **No arbitrary deck support.** Certification reports what the engine implements. Real
  Archidekt Commander decks are currently refused for unsupported cards; the flow's job
  is to name every one of them with a reason.
* Banned/errata status from the provider's `legalities` field is not re-certified, and
  the color-identity check is `not_certified` when the provider sends no identity data.

## Where the code lives

| Path | Purpose |
| --- | --- |
| `crates/import/src/start.rs` | certification (format axis, support axis, seat fit), `plan_start`, URL/text/file entry points |
| `crates/import/src/lib.rs` | unchanged provider guard/fetch/normalize; now also carries provider color identity and type facts |
| `crates/ui/src/session.rs` | `SeatImport` carries both verdicts, the commander and the exact unsupported list; `commander_setup` honours a certified imported commander |
| `crates/ui/src/app.rs` | import panel shows the two axes, every unsupported card, the refusal and the profile it certifies against |

HTTP and disk work stays on the background worker threads (`request_import` /
`poll_workers`); a result whose session generation moved is discarded, and the seat's
busy flag is cleared so the panel cannot be left spinning after a session change.

## Live sample (recorded)

Public deck `https://archidekt.com/decks/27701327` (Commander, owner: public) was fetched
through the ordinary guarded path. Result: one commander (`Seifer, Balamb Rival`,
unsupported), the library certified on both axes, and a refusal naming every unsupported
card with its reason. See `evidence/issue21/` for the recorded certification and the
exact commands.
