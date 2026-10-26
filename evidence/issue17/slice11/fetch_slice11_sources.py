"""Record the slice-11 provenance fixture from live Scryfall.

For each of the two cards this slice added: every printing, the distinct printed
texts among them (there must be exactly one, and it must equal the oracle string
in the compiled catalog), and the instant of retrieval. A card is only claimed as
"sourced" if that equality holds, so the fixture cannot drift from the pool.
"""

from pathlib import Path
import datetime
import json
import re
import urllib.parse
import urllib.request

NAMES = ["Pale Bears", "River Bear"]

# The default printing Scryfall returned for the Ayula gap list this slice drew
# from, so the recorded printing is the one actually used to write the catalog
# entry rather than a remembered set code.
missing = json.loads(
    Path("evidence/issue17/slice11/ayula-missing-printings.json").read_text(encoding="utf-8")
)
default_printing = {
    card["name"]: {"set": card["set"], "collector_number": card["collector_number"]}
    for card in missing["cards"]
    if card["name"] in NAMES
}
assert len(default_printing) == len(NAMES), default_printing

CATALOG = Path("crates/core/src/catalog.rs").read_text(encoding="utf-8")


def oracle_in_catalog(name: str) -> str:
    pattern = re.compile(
        r'card!\(\s*\w+,\s*"' + re.escape(name) + r'".*?oracle:\s*"([^"]*)"', re.DOTALL
    )
    match = pattern.search(CATALOG)
    if not match:
        raise SystemExit(f"{name} is not in the compiled catalog")
    return match.group(1)


def fetch(url: str):
    request = urllib.request.Request(
        url,
        headers={
            "User-Agent": "the-aperture-corpus/1.0",
            "Accept": "application/json",
        },
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.loads(response.read())


def collect(uri: str):
    cards = []
    while uri:
        page = fetch(uri)
        cards.extend(page["data"])
        uri = page.get("next_page")
    return cards


retrieved_at = datetime.datetime.now(datetime.timezone.utc).isoformat()
records = []
for name in NAMES:
    hint = default_printing[name]
    page = fetch(
        "https://api.scryfall.com/cards/search?unique=prints&q=" + urllib.parse.quote(f'!"{name}"')
    )
    printings = page["data"]
    texts = sorted({card.get("oracle_text", "") for card in printings})
    catalog_text = oracle_in_catalog(name)
    assert len(texts) == 1, f"{name} has {len(texts)} distinct printed texts"
    assert texts[0] == catalog_text, (name, texts[0], catalog_text)

    chosen = next(
        (
            card
            for card in printings
            if card["set"] == hint["set"]
            and card["collector_number"] == hint["collector_number"]
        ),
        None,
    )
    assert chosen is not None, f"{name} {hint} printing not found"
    assert "Islandwalk" in chosen["keywords"], chosen["keywords"]
    records.append(
        {
            "name": name,
            "mana_cost": chosen["mana_cost"],
            "mana_value": chosen["cmc"],
            "type_line": chosen["type_line"],
            "oracle_text": chosen["oracle_text"],
            "power": chosen.get("power"),
            "toughness": chosen.get("toughness"),
            "keywords": chosen["keywords"],
            "printing": {"set": chosen["set"], "collector_number": chosen["collector_number"]},
            "printing_set": chosen["set"],
            "prints": len(printings),
            "distinct_printed_texts": len(texts),
            "catalog_oracle_matches_printed": True,
            "scryfall_uri": chosen["scryfall_uri"],
        }
    )
    print(f"{name}: {len(printings)} printings, {len(texts)} distinct text, catalog matches")

fixture = {
    "schema": "aperture.corpus-sources/1",
    "slice": 11,
    "retrieved_at_utc": retrieved_at,
    "source": "https://api.scryfall.com/cards/search?unique=prints",
    "cards": records,
}
out = Path("fixtures/starter-corpus/slice11-sourced.json")
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(fixture, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print("wrote", out)
