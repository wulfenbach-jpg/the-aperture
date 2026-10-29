from pathlib import Path
import shutil
root=Path(__file__).resolve().parents[3]
src=root/'evidence/issue17/slice49-journey-driver'
dst=root/'evidence/issue17/slice50-journey-driver'
(dst/'src').mkdir(parents=True, exist_ok=True)
for name in ('Cargo.toml','Cargo.lock'):
    shutil.copyfile(src/name,dst/name)
s=(src/'src/main.rs').read_text(encoding='utf-8').replace('evidence/issue17/slice49','evidence/issue17/slice50')
s=s.replace('Evidence-only demonstration driver for slice 46','Evidence-only demonstration driver for slice 50')
(dst/'src/main.rs').write_text(s,encoding='utf-8')
s=(root/'evidence/issue17/slice49/corpus-audit.py').read_text(encoding='utf-8').replace('Slice49 corpus recount: Cub repair adds no card.', 'Slice50 corpus recount: Witness conformance repair adds no card.')
(root/'evidence/issue17/slice50/corpus-audit.py').write_text(s,encoding='utf-8')
s=(root/'evidence/issue17/slice49/watch-ci.py').read_text(encoding='utf-8')
(root/'evidence/issue17/slice50/watch-ci.py').write_text(s,encoding='utf-8')
print(dst)
