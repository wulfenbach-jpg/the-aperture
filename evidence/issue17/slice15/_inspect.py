import json
m = json.load(open('decks/starter-corpus/manifest.json'))
lists = m['starters']
for l in lists:
    print(l['id'], l.get('status'), len(l.get('missing_cards', [])))
print()
ay = [l for l in lists if l['id'].startswith('ayula')][0]
names = ay['missing_cards']
print(ay['id'], len(names))
for n in names:
    print(' -', n)
