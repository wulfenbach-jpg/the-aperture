from pathlib import Path
import json
import re
manifest = json.loads(Path('decks/starter-corpus/manifest.json').read_text())
names = set()
for entry in manifest['starters']:
    names.add(entry['commander'])
    for line in Path(entry['deck_path']).read_text().splitlines():
        match = re.match(r'^\s*\d+\s+(.+?)\s*$', line)
        if match:
            names.add(match[1])
pattern = r'card!\(\s*\w+,\s*"([^"]+)"'
catalog = set(re.findall(pattern, Path('crates/core/src/catalog.rs').read_text()))
missing = {name for entry in manifest['starters'] for name in entry['missing_cards']}
result = {
    'catalog_size': len(catalog),
    'exact_corpus_unique_names_including_commanders_and_basics': len(names),
    'currently_supported_corpus_names': len(names & catalog),
    'missing_corpus_names': len(names - catalog),
    'manifest_missing_union': len(missing),
    'missing_union_matches_catalog': missing == names - catalog,
}
assert result['catalog_size'] == 79
assert result['missing_union_matches_catalog']
print(json.dumps(result, indent=2))
Path('evidence/issue17/corpus-union-audit.json').write_text(json.dumps(result, indent=2)+'\n')
