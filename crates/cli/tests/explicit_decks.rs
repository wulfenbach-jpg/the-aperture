use std::process::Command;

#[test]
fn coach_refuses_both_explicit_missing_deck_paths() {
    let missing = std::env::current_dir().unwrap().join(format!(
        "no-such-aperture-coach-deck-{}.txt",
        std::process::id()
    ));
    assert!(!missing.exists());
    for flag in ["--deck-a", "--deck-b"] {
        let output = Command::new(env!("CARGO_BIN_EXE_aperture-cli"))
            .arg("coach")
            .arg(flag)
            .arg(&missing)
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "{flag} silently substituted a bundled deck"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("refusing to substitute"));
    }
}
