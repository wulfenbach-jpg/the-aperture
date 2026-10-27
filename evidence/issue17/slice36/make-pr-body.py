import io

p = "evidence/issue17/slice36/pr-body-before.md"
body = io.open(p, encoding="utf-8").read()

reps = [
    ("- Catalog **149 real cards**.",
     "- Catalog **150 real cards**."),
    ("- Desired eight-list union **459 names = 137 supported + 322 missing** (per-list missing `55/0/51/48/16/52/52/53`).",
     "- Desired eight-list union **459 names = 138 supported + 321 missing** (per-list missing `55/0/51/48/15/52/52/53`)."),
    ("- Nearest list, Ayula: **16 missing names**.",
     "- Nearest list, Ayula: **15 missing names**."),
    ("- Last **behavior** change: slice 33, `Beorn's Hospitality` (first layer 7a characteristic-defining power/toughness, code head `01fdc3b`), repaired in slice 34 (`5b60c2c`) after review 290; `Kamahl's Summons` plus `Surrak and Goreclaw`'s printed trample (slice 32) remain the previous increments.",
     "- Last **behavior** change: slice 35, `Words of Wilding` (the pool's first draw replacement, code head `ac3b422`), whose delivery evidence was completed in slice 36 (`7bb8197`) after review 297; `Beorn's Hospitality` with the review-290 target repair (slices 33-34) remains the previous increment."),
]
for a, b in reps:
    assert a in body, a[:60]
    body = body.replace(a, b, 1)

new_section = """### Latest increment: slice 36 — the draw replacement is now exercised in a real four-seat game

Slice 35 added `Words of Wilding` (`{2}{G}` Enchantment; Scryfall **ONS 305**, immutable `fdb9565f-5b09-4127-b169-3146079dab84`, oracle `888582df-4fe2-4d05-9b62-10c90aaa76f2`, retrieved **2026-10-27T21:06:29Z**) — printed **"{1}: The next time you would draw a card this turn, create a 2/2 green Bear creature token instead."** — as the pool's first **draw replacement** (CR 614.1a): an armed one-shot per-seat count in `GameState::draw_replacements`, consumed by `Game::draw_with_resume` **before the library is read**, so no card moves, no `Event::Draw` is emitted and an empty library is not a loss; cleanup clears an unspent one (CR 514.2); the replacement is not tied to the source's existence.

Review 297 verified the behaviour but blocked the increment on **evidence only**: the committed heuristic journey drew the card three times and never cast it, so a replacement effect was demonstrated by unit tests alone. Slice 36 fixes that with `evidence/issue17/slice35-journey-driver`, a masked demonstrator that plays a real four-seat / 40-life game from the exact `decks/package-slice35.txt` snapshot, casts the card and activates the ability during its own upkeep so that the following draw step consumes the armed replacement. It asserts, from the engine's own events: no `Event::Draw` for that seat, no `DrewFromEmpty`, library and hand sizes unchanged, exactly one new 2/2 green Bear **creature** token, the armed count down by exactly one, no loss. It also exercises the unspent half in the same game and asserts the conservation law `armed = replaced + cleared + leftover`.

Result: terminal `Win{P3}`, 2048 steps, 41 turns, 5347 replay events, digest `1c13d8fdcdce...`, **Words of Wilding cast 4 times, 124 activations, 25 replaced draws (by seat `[3,3,9,10]`), 94 spare arms cleared at cleanup, 5 left when the game ended (124 = 25 + 94 + 5), 25 Bear tokens, 0 `DrewFromEmpty`**. Artifacts committed byte-identical on re-run (`driver-reproducibility.log`) and replayed by the release CLI independently (`driver-cli-replay.log`). Slice 36 also corrects the effect's player-facing wording to the printed "you", restores slice 33's own-time Ayula count, and discloses the commander-draw ordering limit (the replacement is consumed before the CR 903.9b suspension with no CR 616.1 choice surface; unreachable today). Gates at `7bb8197`: fmt and `clippy -D warnings` clean, `issue17_slice35` 6/6, `card_printings` 13/13, workspace **120 suites / 853 passed / 0 failed**, and **10/10 exact-head CI SUCCESS keyed to `7bb8197`** (the earlier docs tip `ec8dac2` also carries its own 10/10). Checkpoint: `evidence/issue17/SLICE36-HUMAN-CHECKPOINT.md`.

"""
marker = "### Latest increment: slice 33 — Beorn's Hospitality, with the review-290 target repair in slice 34"
assert marker in body
body = body.replace(marker, new_section + marker, 1)

io.open("evidence/issue17/slice36/pr-body-after.md", "w", encoding="utf-8", newline="\n").write(body)
print(len(body.splitlines()), "lines")
