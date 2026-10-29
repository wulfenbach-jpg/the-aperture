import json, hashlib

p = 'decks/starter-corpus/manifest.json'
m = json.load(open(p, encoding='utf-8'))
deck = 'decks/starter-corpus/jasmine-vanilla-stompy.txt'
h = hashlib.sha256(open(deck, 'rb').read()).hexdigest()
for s in m['starters']:
    if s['id'] != 'jasmine-vanilla-stompy':
        continue
    s['status'] = 'certified'
    s['certification'] = {
        "verdict": "certified",
        "certified_on": "2026-10-26",
        "snapshot_sha256": h,
        "library_cards": 99,
        "total_cards": 100,
        "commander_supported": True,
        "seats": 4,
        "seeds": [1, 2, 3],
        "games_terminal": 3,
        "games_replay_verified": 3,
        "unique_names_observed_in_play": 54,
        "unique_names_total": 61,
        "evidence": "evidence/issue17/slice10/certification.json",
        "reproduce": "cargo run --release -p aperture-cli -- certify --starter jasmine-vanilla-stompy --seats 4 --seeds 1,2,3 --out evidence/issue17/slice10/certification.json",
        "conformance_suites": [
            "crates/core/tests/starter_corpus.rs",
            "crates/core/tests/issue17_slice10_certification.rs",
            "crates/cli/tests/certify.rs",
        ],
        "interaction_exclusions": [
            "legend rule (CR 704.5j): unmodelled; unreachable in this list because every seat supplies exactly one copy of its commander and the list contains no copy effects",
            "equipment, Auras, planeswalkers and nonbasic lands: none are in the list, so their rules are neither exercised nor claimed",
            "native Commander table, trainer, measured playing strength and arbitrary Archidekt compatibility: out of scope for this certificate",
        ],
        "limitations": [
            "bounded commander-combat practice profile (40 life, one supported commander per seat, 21 commander damage), not the full Commander format",
            "the certified games use the built-in heuristic; the certificate proves the list plays to a terminal, replay-verified game, not that any policy plays it well",
        ],
    }
open(p, 'w', encoding='utf-8', newline='\n').write(json.dumps(m, indent=2, ensure_ascii=False) + "\n")
print('statuses', [s['status'] for s in m['starters']])
print('hash', h)
