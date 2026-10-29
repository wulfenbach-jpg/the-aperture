from pathlib import Path

p = Path('README.md')
s = p.read_text(encoding='utf-8')
old = "  Crack the Earth is described here:"
new = """  Slice 59 adds **Dúnedain Rangers** (`{3}{G}` Human Ranger 4/4, LTR 159) with its
  printed landfall Ring-temptation trigger, and the engine-only **The Ring** emblem
  (never importable): Ring-bearer designation, CR 704.5j legendary status, the
  greater-power block restriction, attack draw-then-discard, end-of-combat blocker
  sacrifice and combat-damage life loss at levels 1-4. See
  [docs/issue17-starter-corpus.md](docs/issue17-starter-corpus.md) §Slice 59.
  Crack the Earth is described here:"""
assert s.count(old) == 1
p.write_text(s.replace(old, new), encoding='utf-8', newline='\n')
print('README pool paragraph updated')
