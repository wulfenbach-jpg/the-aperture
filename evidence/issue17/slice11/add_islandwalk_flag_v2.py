"""Give `CardDef` a real `islandwalk` flag (v2, line-ending agnostic).

The pool guard (`every_printed_ability_maps_to_implemented_semantics`) requires a
card whose entire printed text is a keyword to have the `CardDef` field the engine
executes, so the keyword belongs in the struct rather than only in the blocking
table. Every existing `CardDef` literal gets `islandwalk: false`, the two new cards
use a dedicated macro, and `tokens::keywords` keeps serving the blocker check.
"""

import io
import re


def read(path):
    s = io.open(path, encoding="utf-8", newline="").read()
    return s, ("\r\n" if "\r\n" in s else "\n")


def write(path, s):
    io.open(path, "w", encoding="utf-8", newline="").write(s)


card_rs = "crates/core/src/card.rs"
s, nl = read(card_rs)
anchor = nl.join(
    [
        "    /// CR 702.19b: excess combat damage may be assigned to the defending",
        "    /// player once every blocker has been assigned lethal damage.",
        "    pub trample: bool,",
        "",
    ]
)
assert s.count(anchor) == 1, "trample field anchor"
s = s.replace(
    anchor,
    nl.join(
        [
            "    /// CR 702.19b: excess combat damage may be assigned to the defending",
            "    /// player once every blocker has been assigned lethal damage.",
            "    pub trample: bool,",
            "    /// CR 702.14a: printed islandwalk. A blocking restriction, executed by",
            "    /// `tokens::can_block` through the keyword table; nothing grants it.",
            "    pub islandwalk: bool,",
            "",
        ]
    ),
)
accessor = nl.join(["    pub fn has_trample(&self) -> bool {", "        self.trample", "    }", ""])
assert s.count(accessor) == 1, "trample accessor anchor"
s = s.replace(
    accessor,
    nl.join(
        [
            "    pub fn has_trample(&self) -> bool {",
            "        self.trample",
            "    }",
            "",
            "    /// CR 702.14a: \"This creature can't be blocked as long as defending",
            "    /// player controls an Island.\"",
            "    pub fn has_islandwalk(&self) -> bool {",
            "        self.islandwalk",
            "    }",
            "",
        ]
    ),
)
write(card_rs, s)
print("card.rs: field + accessor added")

for path in ("crates/core/src/catalog.rs", "crates/core/src/tokens.rs"):
    s, nl = read(path)
    s, count = re.subn(
        r"( +)(trample: (?:true|false),)(\r?\n)",
        lambda m: f"{m.group(1)}{m.group(2)}{m.group(3)}{m.group(1)}islandwalk: false,{m.group(3)}",
        s,
    )
    write(path, s)
    print(f"{path}: {count} CardDef literals updated")

catalog_rs = "crates/core/src/catalog.rs"
s, nl = read(catalog_rs)
marker = nl.join(["/// A creature that prints Trample (CR 702.19) alongside its other text.", ""])
assert s.count(marker) == 1, "trample macro doc anchor"
macro_lines = [
    "/// A creature whose only printed rules text is Islandwalk (CR 702.14a).",
    "macro_rules! card_islandwalk {",
    "    (",
    "        $id:ident, $name:literal, cost: $cost:expr, kinds: $kinds:expr, basic: $basic:expr,",
    "        type_line: $tl:literal, power: $power:expr, toughness: $toughness:expr,",
    "        oracle: $oracle:literal, spell: $spell:expr",
    "    ) => {",
    "        CardDef {",
    "            id: CardId::$id,",
    "            name: $name,",
    "            cost: $cost,",
    "            kinds: $kinds,",
    "            basic_mana: $basic,",
    "            type_line: $tl,",
    "            colors: &[],",
    "            flying: false,",
    "            defender: false,",
    "            flash: false,",
    "            trample: false,",
    "            islandwalk: true,",
    "            power: $power,",
    "            toughness: $toughness,",
    "            oracle: $oracle,",
    "            spell: $spell,",
    "        }",
    "    };",
    "}",
    "",
    "",
]
s = s.replace(marker, nl.join(macro_lines))

for card in ("PaleBears", "RiverBear"):
    pattern = re.compile(r"    card!\(\r?\n        " + card + r", ")
    s, count = pattern.subn(f"    card_islandwalk!({nl}        {card}, ", s)
    assert count == 1, card
write(catalog_rs, s)
print("catalog.rs: card_islandwalk! macro added and used")
