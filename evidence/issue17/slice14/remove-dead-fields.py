import pathlib
import re

# Remove the dead CardDef fields: every constructor set them to None and every
# consumer reads the CardId accessors instead.
for name in ['crates/core/src/catalog.rs', 'crates/core/src/tokens.rs']:
    path = pathlib.Path(name)
    text = path.read_text(encoding='utf-8')
    before = text.count('mana_choices: None,')
    text = re.sub(r'[ \t]*mana_choices: None,\r?\n', '', text)
    text = re.sub(r'[ \t]*creature_mana: None,\r?\n', '', text)
    path.write_text(text, encoding='utf-8')
    print(name, 'removed', before, 'field initializers')

path = pathlib.Path('crates/core/src/card.rs')
text = path.read_text(encoding='utf-8')
old = '''    /// Dynamic one-mana ability: the offered colors are computed from live state.
    pub mana_choices: Option<ManaChoice>,
    /// Tap mana ability on a creature; obeys summoning sickness.
    pub creature_mana: Option<ManaColor>,
'''
assert text.count(old) == 1
path.write_text(text.replace(old, ''), encoding='utf-8')
print('removed dead CardDef field declarations')
