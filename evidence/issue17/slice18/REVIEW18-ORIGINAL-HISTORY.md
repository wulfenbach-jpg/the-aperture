# Slice 18 — true original-history adjudication

**CHANGES_REQUESTED** at `2785c2cefbb92b678ace58949d2373f982a4a208`.

This is the true original reviewer history (the earlier slice17 launch used the
wrong review-sessions path; that separate review was read as evidence, then the
focused checks below were re-executed).

## Blocking finding — tracked records are wrong

My independent all-name audit gives **135 catalog / 459 union / 123 supported /
336 missing**, with per-list missing `55/0/51/48/30/52/52/53`; manifest arrays
match and no missing name is in the catalog. The handoff has the correct
123/336, but `docs/issue17-starter-corpus.md:49` still says **122 supported /
337 missing** and cites the slice13 audit. The same doc also still says Ayula's
counter/tribal text is not implemented (`:175`) and lists Ayula among three
unsupported commanders (`:315`), although Ayula is implemented and registered.
The code is fine; the records must be corrected to 123/336 and Ayula-supported.
The checkpoint's 454/118 parse omitted names and is not the all-name union.

Reproduction: manifest union = 336, catalog membership = 123; docs line 49 says
122/337. `cargo test -p aperture-core --test issue17_slice18` is not the
discriminator; the audit/manifest read is.

## Executed / inspected

- `cargo test -p aperture-core --test abilities --locked`: **13/13**, including
  `optional_trigger_is_declined_or_used_by_its_controller`.
- `cargo test -p aperture-core --test command_zone_setup --locked`: **44/44**
  including Ayula mode/target, fight prevention/target-departure, source-left,
  save/reload, and forged-submission controls.
- `cargo test -p aperture-core --lib --locked`: **13/13**, including the
  queued-trigger batch regression.
- `cargo test -p aperture-core --test rules_conformance`: **5/5**;
  `issue17_slice17`: **6/6**; fmt/diff-check pass.
- Demo replay: 4 seats, 1961 actions, terminal Win P2, 5028 events, digest
  `09e6bb25c505…`; `sha256(decks/package-slice18.txt)` matches the recorded
  `40028e6e…` snapshot. Save replay reproduces the same digest.
- Source fixture has immutable printing URL/retrieval timestamps. No
  import/UI/AI-policy production edit; no desired 100-card replacement.

## Scope

Full original #17 is unfinished (**1/8 certified**, 336 missing union names).
This is not whole-issue acceptance, merge, closure, or downstream unlock.
