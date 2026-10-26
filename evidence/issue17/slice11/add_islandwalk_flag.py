"""Give `CardDef` a real `islandwalk` flag.

The pool guard (`every_printed_ability_maps_to_implemented_semantics`) requires a
card whose entire printed text is a keyword to have the `CardDef` field the engine
executes, so the keyword belongs in the struct rather than only in the blocking
table. Every existing `CardDef` literal gets `islandwalk: false`, the two new cards
use a dedicated macro, and `tokens::keywords` keeps serving the blocker check.
"""

import io
import re

# 1. The struct field and its accessor.
card_rs = "crates/core/src/card.rs"
s = io.open(card_rs, encoding="utf-8", newline="").read()
anchor = (
    "    /// CR 702.19b: excess combat damage may be assigned to the defending\r\n"
    "    /// player once every blocker has been assigned lethal damage.\r\n"
    "    pub trample: bool,\r\n"
)
assert s.count(anchor) == 1
s = s.replace(
    anchor,
    anchor
    + "    /// CR 702.14a: printed islandwalk. A blocking restriction, executed by\r\n"
    "    /// `tokens::can_block` through the keyword table; nothing grants it.\r\n"
    "    pub islandwalk: bool,\r\n",
)
accessor = (
    "    pub fn has_trample(&self) -> bool {\r\n        self.trample\r\n    }\r\n"
)
assert s.count(accessor) == 1
s = s.replace(
    accessor,
    accessor
    + "\r\n    /// CR 702.14a: \"This creature can't be blocked as long as defending\r\n"
    "    /// player controls an Island.\"\r\n"
    "    pub fn has_islandwalk(&self) -> bool {\r\n        self.islandwalk\r\n    }\r\n",
)
io.open(card_rs, "w", encoding="utf-8", newline="").write(s)
print("card.rs: field + accessor added")

# 2. Every existing CardDef literal keeps the keyword off.
for path in ("crates/core/src/catalog.rs", "crates/core/src/tokens.rs"):
    s = io.open(path, encoding="utf-8", newline="").read()
    matches = re.findall(r"( +)trample: (?:true|false),(?:\r?\n)", s)
    s, count = re.subn(
        r"( +)(trample: (?:true|false),)(\r?\n)",
        lambda m: f"{m.group(1)}{m.group(2)}{m.group(3)}{m.group(1)}islandwalk: false,{m.group(3)}",
        s,
    )
    io.open(path, "w", encoding="utf-8", newline="").write(s)
    print(f"{path}: {count} CardDef literals updated")

# 3. A dedicated macro for the two islandwalk cards, modelled on card_flash!.
catalog_rs = "crates/core/src/catalog.rs"
s = io.open(catalog_rs, encoding="utf-8", newline="").read()
marker = "/// A creature that prints Trample (CR 702.19) alongside its other text.\r\n"
assert s.count(marker) == 1
macro_text = (
    "/// A creature whose only printed rules text is Islandwalk (CR 702.14a).\r\n"
    "macro_rules! card_islandwalk {\r\n"
    "    (\r\n"
    "        $id:ident, $name:literal, cost: $cost:expr, kinds: $kinds:expr, basic: $basic:expr,\r\n"
    "        type_line: $tl:literal, power: $power:expr, toughness: $toughness:expr,\r\n"
    "        oracle: $oracle:literal, spell: $spell:expr\r\n"
    "    ) => {\r\n"
    "        CardDef {\r\n"
    "            id: CardId::$id,\r\n"
    "            name: $name,\r\n"
    "            cost: $cost,\r\n"
    "            kinds: $kinds,\r\n"
    "            basic_mana: $basic,\r\n"
    "            type_line: $tl,\r\n"
    "            colors: &[],\r\n"
    "            flying: false,\r\n"
    "            defender: false,\r\n"
    "            flash: false,\r\n"
    "            trample: false,\r\n"
    "            islandwalk: true,\r\n"
    "            power: $power,\r\n"
    "            toughness: $toughness,\r\n"
    "            oracle: $oracle,\r\n"
    "            spell: $spell,\r\n"
    "        }\r\n"
    "    };\r\n"
    "}\r\n\r\n"
)
s = s.replace(marker, macro_text + marker)

# The two new entries use it.
for card in ("PaleBears", "RiverBear"):
    pattern = re.compile(r"    card!\(\r?\n        " + card + r", ")
    s, count = pattern.subn(f"    card_islandwalk!(\r\n        {card}, ", s)
    assert count == 1, card
io.open(catalog_rs, "w", encoding="utf-8", newline="").write(s)
print("catalog.rs: card_islandwalk! macro added and used")
