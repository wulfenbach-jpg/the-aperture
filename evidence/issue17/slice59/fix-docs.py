from pathlib import Path

p = Path('docs/issue17-starter-corpus.md')
s = p.read_text(encoding='utf-8')
old = """1. Ring level 1: the Ring-bearer is effectively legendary and cannot be blocked by a
   creature with greater power. The legend SBA groups all same-name permanents when any
   member is effectively legendary, and only the group's controller answers."""
new = """1. Ring level 1: the Ring-bearer is effectively legendary and cannot be blocked by a
   creature with greater power. The legend SBA counts only legendary permanents with the
   same name (printed legendary, or the controller's Ring-bearer), so one legend beside an
   ordinary same-name card is not a legend-rule choice; only that group's controller answers."""
assert s.count(old) == 1, s.count(old)
p.write_text(s.replace(old, new), encoding='utf-8', newline='')

q = Path('docs/issue17-starter-corpus.md')
t = q.read_text(encoding='utf-8', newline='')
a = "1. **Card-support surface.** The implemented pool is 164 cards."
b = "1. **Card-support surface.** The implemented pool is 165 cards."
assert t.count(a) == 1
q.write_text(t.replace(a, b), encoding='utf-8', newline='')
print('docs updated')
