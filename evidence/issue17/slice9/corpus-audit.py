import pathlib,json,re,subprocess,collections
r=pathlib.Path('.')
m=json.loads((r/'decks/starter-corpus/manifest.json').read_text())
cat={n:c for c,n in re.findall(r'card(?:!|_[a-z]+!)\(\s*(\w+),\s*"([^"]+)"',(r/'crates/core/src/catalog.rs').read_text())}
u=set(); miss=set(); rows=[]
for s in m['starters']:
    names={s['commander']}
    cards=collections.Counter()
    for l in (r/s['deck_path']).read_text().splitlines():
        if re.match(r'^\d+ ',l):
            count,name=l.split(maxsplit=1); cards[name]+=int(count); names.add(name)
    assert sum(cards.values())==99
    original=subprocess.check_output(['git','show','e16185c:'+s['deck_path']]).decode().replace('\r\n','\n')
    assert original==(r/s['deck_path']).read_text().replace('\r\n','\n')
    assert names-cat.keys()==set(s['missing_cards'])
    u.update(names);miss.update(s['missing_cards']);rows.append({'id':s['id'],'missing':len(s['missing_cards']),'status':s['status']})
result={'catalog':len(cat),'union':len(u),'supported':len(u&cat.keys()),'missing':len(u-cat.keys()),'manifest_missing':len(miss),'matches':miss==u-cat.keys(),'rows':rows,'desired_contents_unchanged':True}
print(json.dumps(result,indent=2));(r/'evidence/issue17/slice9/corpus-state.json').write_text(json.dumps(result,indent=2)+'\n')
