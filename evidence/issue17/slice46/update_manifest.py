import json

path = 'decks/starter-corpus/manifest.json'
with open(path, encoding='utf-8') as handle:
    manifest = json.load(handle)

ayula = next(s for s in manifest['starters'] if s['id'] == 'ayula-bear-counters')
ayula['missing_cards'] = [c for c in ayula['missing_cards'] if c != 'Thijarian Witness']
ayula['missing_interactions'] = [
    entry
    for entry in ayula['missing_interactions']
    if 'Thijarian Witness' not in entry
]

with open(path, 'w', encoding='utf-8', newline='\n') as handle:
    json.dump(manifest, handle, indent=1, ensure_ascii=False)
    handle.write('\n')

print(json.dumps(ayula, indent=1, ensure_ascii=False))
