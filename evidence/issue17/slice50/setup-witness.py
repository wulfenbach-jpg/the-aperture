from pathlib import Path
import shutil
root=Path(__file__).resolve().parents[3]
src=root/'evidence/issue17/slice46-journey-driver'
dst=root/'evidence/issue17/slice50-witness-driver'
(dst/'src').mkdir(parents=True, exist_ok=True)
for name in ('Cargo.toml','Cargo.lock'):
    shutil.copyfile(src/name,dst/name)
s=(src/'src/main.rs').read_text(encoding='utf-8')
s=s.replace('Evidence-only demonstration driver for slice 46','Evidence-only demonstration driver for slice 50')
s=s.replace('evidence/issue17/slice46','evidence/issue17/slice50/witness')
(dst/'src/main.rs').write_text(s,encoding='utf-8')
print(dst)
