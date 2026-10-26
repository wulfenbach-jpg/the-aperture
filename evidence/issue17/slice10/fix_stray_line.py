p = 'crates/core/tests/issue17_slice10_certification.rs'
lines = open(p, encoding='utf-8', newline='').read().split(chr(10))
kept = [line for line in lines if not line.startswith(chr(92) + 'n",')]
open(p, 'w', encoding='utf-8', newline='').write(chr(10).join(kept))
print('removed', len(lines) - len(kept), 'stray line(s)')
