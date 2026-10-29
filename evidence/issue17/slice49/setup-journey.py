from pathlib import Path
import shutil
root = Path(__file__).resolve().parents[3]
src = root / 'evidence/issue17/slice48-cub-journey-driver'
dst = root / 'evidence/issue17/slice49-journey-driver'
(dst / 'src').mkdir(parents=True, exist_ok=True)
for name in ('Cargo.toml', 'Cargo.lock'):
    shutil.copyfile(src / name, dst / name)
text = (src / 'src/main.rs').read_text(encoding='utf-8')
text = text.replace('let mut alone_attacks = 0usize;', 'let mut alone_attacks = 0usize;\n    let mut observed_private_looks = 0usize;\n    let mut viewed_card_samples = 0usize;\n    let mut acknowledgements = 0usize;')
text = text.replace('let revision = game.revision();', '''if matches!(action, Action::SearchLibraryDone) {
            assert!(!view.pending_viewed.is_empty(), "the controller must see examined identities");
            assert!(view.pending_viewed.len() <= 8);
            observed_private_looks += 1;
            viewed_card_samples += view.pending_viewed.len();
            if view.legal.len() == 1 { acknowledgements += 1; }
            for other in 0..4 {
                if other != player {
                    assert!(game.view_for(other, &events).pending_viewed.is_empty());
                }
            }
        }
        let revision = game.revision();''')
text = text.replace('evidence/issue17/slice48-cub', 'evidence/issue17/slice49')
text = text.replace('"cub_looks": looks,', '"cub_private_looks_observed": observed_private_looks,\n        "private_viewed_card_samples": viewed_card_samples,\n        "done_only_acknowledgements": acknowledgements,\n        "cub_looks": looks,')
text = text.replace('let report = serde_json::json!({', 'assert_eq!(observed_private_looks, looks);\n    let report = serde_json::json!({')
(dst / 'src/main.rs').write_text(text, encoding='utf-8')
print(dst)
