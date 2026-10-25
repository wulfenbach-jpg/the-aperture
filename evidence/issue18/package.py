"""Rebuild the portable issue18 package from the already-built release binaries."""
from pathlib import Path
import hashlib
import json
import shutil
import zipfile

root = Path(__file__).resolve().parents[2]
pkg = root / 'dist/aperture-issue18-native-table'
pkg.mkdir(parents=True, exist_ok=True)
for name in ('aperture.exe', 'aperture-cli.exe'):
    shutil.copy2(root/'target/release'/name, pkg/name)
for name in ('LICENSE-MIT', 'LICENSE-APACHE', 'ASSETS.md'):
    shutil.copy2(root/name, pkg/name)
shutil.copytree(root/'decks', pkg/'decks', dirs_exist_ok=True)
(pkg/'LAUNCH.cmd').write_text('@echo off\ncd /d "%~dp0"\nstart "" aperture.exe --seats 4 --profile bounded --autoplay\n', encoding='utf-8')
(pkg/'README.txt').write_text('THE APERTURE - bounded native practice\n\nExtract the entire ZIP, then double-click LAUNCH.cmd. Human P0 versus three heuristic AI seats. No model service/GPU required.\n\nDefault launcher: four seats, 40 life, one supported Isamaru commander per seat. Select ordinary 20 life or 2/3/4 seats in the app and press Start new table with these options. Direct aperture.exe retains the two-seat20-life default.\n\nTab/Shift+Tab: focus; Enter: activate. H: keyboard help. A-/A+: text zoom. Public-zone buttons and cards: inspect. S: async save; L: async load. Files are written beside the app when launched through LAUNCH.cmd.\n\n28-card practice pool only. NOT arbitrary Commander, 100-card/color-identity/legend-rule/partners support, trainer or v1. Sandbox counters are stored/displayed only, not rules-supported counters. Automatic combat damage assignment is a disclosed simplification. Unsigned Windows build; no model weights or private configuration included. See source docs/issue18-native-table.md.\n',encoding='utf-8')
z = root/'dist/aperture-issue18-native-table.zip'
with zipfile.ZipFile(z,'w',zipfile.ZIP_DEFLATED) as out:
    for p in sorted(pkg.rglob('*')):
        if p.is_file() and p.suffix not in ('.json', '.log'):
            out.write(p,p.relative_to(pkg))
with zipfile.ZipFile(z) as check:
    assert check.testzip() is None
    entries = check.namelist()
manifest = dict(zip_sha256=hashlib.sha256(z.read_bytes()).hexdigest(),zip_bytes=z.stat().st_size,entries=entries,exe_sha256=hashlib.sha256((pkg/'aperture.exe').read_bytes()).hexdigest())
(root/'evidence/issue18/package-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps(manifest,indent=2))
