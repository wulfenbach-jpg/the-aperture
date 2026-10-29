from pathlib import Path
import subprocess, json, urllib.request
root=Path('C:/AI/aperture-worktrees/issue-17')
out=root/'evidence/issue17/review286'
repo=out/'repro'
driver=repo/'evidence/issue17/slice29-journey-driver'
for rel in ['Cargo.toml','Cargo.lock','src/main.rs']:
    data=subprocess.check_output(['git','show','de3ba07:evidence/issue17/slice29-journey-driver/'+rel],cwd=root)
    dest=driver/rel; dest.parent.mkdir(parents=True,exist_ok=True)
    if rel=='Cargo.toml':
        data=data.replace(b'../../../crates/core',b'C:/AI/aperture-worktrees/issue-17/crates/core').replace(b'../../../crates/ai',b'C:/AI/aperture-worktrees/issue-17/crates/ai')
    dest.write_bytes(data)
rel='decks/package-slice29.txt'
dest=repo/rel; dest.parent.mkdir(parents=True,exist_ok=True)
dest.write_bytes(subprocess.check_output(['git','show','de3ba07:'+rel],cwd=root))
# A separate instrumented copy checks real mid-animation session save/load.
probe=repo/'evidence/issue17/mid-animation-probe'
for rel in ['Cargo.toml','Cargo.lock','src/main.rs']:
    dest=probe/rel; dest.parent.mkdir(parents=True,exist_ok=True)
    data=(driver/rel).read_bytes()
    if rel=='src/main.rs':
        text=data.decode()
        text=text.replace('let mut steps = 0;', 'let mut steps = 0; let mut mid_checked = false;')
        needle='        steps += 1;'
        code='''        if !mid_checked && game.state.effects.iter().any(|e| matches!(e.kind, aperture_core::layer::ContinuousKind::AddCreatureSubtypes(_))) {
            let saved = SessionSave::capture(&game, replay.clone(), Vec::new(), false).to_json();
            let loaded = SessionSave::from_json(&saved).expect("real mid-animation save must validate");
            assert_eq!(loaded.game, game.state);
            assert!(game.state.objects.values().any(|o| o.card == CardId::StuffedBear && game.is_creature(o)));
            mid_checked = true;
            println!("MID_ANIMATION_SESSION_VALIDATION_PASS steps={}", steps + 1);
        }
'''
        assert needle in text
        text=text.replace(needle,code+needle).replace('assert!(game.state.is_over(), "capped, not terminal");','assert!(game.state.is_over(), "capped, not terminal"); assert!(mid_checked);')
        data=text.encode()
    dest.write_bytes(data)
for printing in ['mid/259','dbl/259']:
    request=urllib.request.Request('https://api.scryfall.com/cards/'+printing, headers={'User-Agent':'ApertureReview/1.0','Accept':'application/json'})
    try:
        with urllib.request.urlopen(request,timeout=30) as r: obj=json.load(r)
    except Exception as exc:
        print('LIVE_SOURCE_UNAVAILABLE', printing, str(exc)); continue
    (out/(printing.replace('/','-')+'-live.json')).write_text(json.dumps(obj,indent=2),encoding='utf-8')
    print(printing,obj['name'],obj['oracle_text'],obj['type_line'],obj['mana_cost'])
print('Prepared exact committed driver and a separately labelled mid-animation save probe')
