"""Adversarial offline snapshot probes; never modifies repo fixtures."""
import copy
import hashlib
import json
import pathlib
import subprocess

PHASE = pathlib.Path(__file__).parent
REPO = pathlib.Path('C:/AI/aperture-worktrees/issue-11')
FIXTURES = REPO / 'crates/import/tests/fixtures/commander/real'
OUT = PHASE / 'supervisor-probes'
OUT.mkdir(exist_ok=True)
BASE = json.loads((FIXTURES / 'cards-scryfall.json').read_text(encoding='utf8'))
CARDS = [json.loads(line) for line in BASE['raw_cards_jsonl'].splitlines() if line.strip()]
BIN = REPO / 'target/debug/aperture-cli.exe'

def run(name, cards):
    wrapper = copy.deepcopy(BASE)
    raw = ''.join(json.dumps(card, ensure_ascii=False, separators=(',', ':')) + '\n' for card in cards)
    wrapper['raw_cards_jsonl'] = raw
    wrapper['provenance']['cards_sha256'] = hashlib.sha256(raw.encode()).hexdigest()
    # Assignment-5 truthful provenance: raw_sha256 binds the same embedded bytes.
    wrapper['provenance']['raw_sha256'] = wrapper['provenance']['cards_sha256']
    wrapper['provenance']['card_count'] = len(cards)
    wrapper['provenance']['note'] = 'ADVERSARIAL MUTATION; not a genuine captured Scryfall fixture'
    path = OUT / (name + '.json')
    path.write_text(json.dumps(wrapper), encoding='utf8')
    result = subprocess.run([str(BIN), 'commander-check', '--deck', str(FIXTURES / 'deck-legal-100.json'), '--cards', str(path), '--json'], capture_output=True, text=True)
    (OUT / (name + '.stdout.json')).write_text(result.stdout, encoding='utf8')
    (OUT / (name + '.stderr.txt')).write_text(result.stderr, encoding='utf8')
    try:
        report = json.loads(result.stdout)
    except ValueError:
        report = {}
    return {'probe': name, 'returncode': result.returncode, 'status': report.get('status'), 'findings': [f for f in report.get('findings', []) if f.get('status') != 'legal']}

results = [run('valid-control', CARDS)]
modified = copy.deepcopy(CARDS[0])
modified['color_identity'] = ['U']
results.append(run('conflicting-same-oracle-append', CARDS + [modified]))
results.append(run('conflicting-same-oracle-prepend', [modified] + CARDS))
modified = copy.deepcopy(CARDS)
modified[0]['oracle_text'] = None
results.append(run('commander-null-oracle-text', modified))
modified = copy.deepcopy(CARDS)
modified[0]['type_line'] = 'Fictional Legendary Creature'
results.append(run('commander-unknown-type-token', modified))
(OUT / 'results.json').write_text(json.dumps(results, indent=2), encoding='utf8')
print(json.dumps(results, indent=2))
