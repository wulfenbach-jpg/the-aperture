import re
from pathlib import Path

txt = Path('crates/core/src/catalog.rs').read_text(encoding='utf-8')
start = txt.index('pub static CARDS')
end = txt.index('pub fn all()')
block = txt[start:end]
print('card! occurrences in CARDS block', len(re.findall(r'card!\(', block)))
print('CardDef literals in CARDS block', len(re.findall(r'CardDef\s*\{', block)))
names = re.findall(r'card!\(\s*\w+\s*,\s*"([^"]+)"', block)
print('parsed via card!', len(names))
bad = []
for m in re.finditer(r'card!\((.{0,150})', block, re.S):
    seg = m.group(1)
    if not re.match(r'\s*\w+\s*,\s*"', seg):
        bad.append(seg[:150].replace('\n', ' '))
print('unparsed card! count', len(bad))
for b in bad[:25]:
    print('   ', b)
