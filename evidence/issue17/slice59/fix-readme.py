from pathlib import Path

p = Path('README.md')
s = p.read_text(encoding='utf-8')
pairs = [
(
"""  Isamaru, **Jasmine Boreal**, **Krenko, Mob Boss** and **Talrand, Sky Summoner**
  are commander-only; they
  are refused in every ordinary starting library because the legend rule is not
  implemented.""",
"""  Isamaru, **Jasmine Boreal**, **Krenko, Mob Boss** and **Talrand, Sky Summoner**
  are commander-only; they are refused in every ordinary starting library because
  the named command-zone profile owns their setup. The legend rule itself
  (CR 704.5j, same-name legendary permanents controlled by one player) is
  implemented as a state-based action."""
),
(
"""Boreal are the supported commanders, there is no color-identity/100-card profile, no
commander damage, no legend rule, no partner/multi-commander; the native table""",
"""Boreal are the supported commanders, there is no color-identity/100-card profile, no
commander damage and no partner/multi-commander (the CR 704.5j legend rule is
implemented separately); the native table"""
),
(
"""combat-only credit). It is still **not the Commander format** (no 100-card
construction, partners, legend rule or full elimination/APNAP cleanup). The""",
"""combat-only credit). It is still **not the Commander format** (no 100-card
construction, partners or full elimination/APNAP cleanup; the legend rule is
implemented as its own state-based action). The"""
),
(
"""Full Commander legality and gameplay — the bounded command-zone and
Commander-combat practice profiles above are not Commander: color identity,
100-card singleton construction, the legend rule and partners/multi-commander
are absent.""",
"""Full Commander legality and gameplay — the bounded command-zone and
Commander-combat practice profiles above are not Commander: color identity,
100-card singleton construction and partners/multi-commander are absent. (The
CR 704.5j legend rule is implemented as a state-based action; it is not a
Commander-construction feature.)"""
),
(
"""  requirement, indestructible in the state-based-action path, announced `{X}` costs
  with X carried through resolution, aggregate \"total mana value X or less\" target
  sets, bounded combat with""",
"""  requirement, indestructible in the state-based-action path, announced `{X}` costs
  with X carried through resolution, aggregate \"total mana value X or less\" target
  sets, the CR 704.5j legend rule, The Ring emblem with its four level-gated
  triggered abilities (including Ring-bearer designation and landfall temptation),
  bounded combat with"""
),
]
for old, new in pairs:
    assert s.count(old) == 1, old[:60]
    s = s.replace(old, new)
p.write_text(s, encoding='utf-8', newline='\n')
print('README updated')
