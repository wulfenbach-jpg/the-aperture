# Security audit — dependencies, untrusted inputs, third-party distribution

Lane: issue #31 security audit · Date: 2026-10-26 · Status: audit of the
integrated baseline, one bounded slice. An audit of this commit is **not**
perpetual certification or a v1 approval.

## Audited scope (exact)

| Item | Value |
| --- | --- |
| Audited head | `055fde7bc2ff296dc1d94c07dfd566fc7082bdf2` (master state after accepted PR #47 integration) |
| Worktree | `C:/AI/aperture-worktrees/issue-31`, branch `aperture/issue-31`, clean tree |
| Dependencies scanned | 498 locked crates (`Cargo.lock`, unchanged in this lane) |
| Advisory DB | RustSec via cargo-audit, 1352 advisories, fetched 2026-10-26 |
| Host | Windows 10, rustc 1.97.1 (8bab26f4f 2026-07-14), cargo 1.97.1 |
| Tools | cargo-audit 0.22.2, cargo-deny 0.20.2 (built into a scratch-local tool root, no global installs), scanner `scripts/security/scan_credentials.py` (CPython 3.14.9, stdlib only) |
| Out of scope, incoming | #17 corpus delta (PR #46 lane) and #25 model lane are active on other branches — **require versioned recheck at final packaging/release** |

## 1. Dependency and advisory audit

Command (exact):

```
C:/tmp/audit-tools/tools/bin/cargo-audit.exe audit
C:/tmp/audit-tools/tools/bin/cargo-deny.exe --config scripts/security/deny.toml check licenses
```

Result: **0 vulnerabilities** in 498 dependencies. 5 hygiene warnings, all
low severity, all transitive to the UI stack, none with a known exploit:

| Crate | Advisory | Class | Triage |
| --- | --- | --- | --- |
| ttf-parser 0.25.1 | RUSTSEC-2026-0192 | unmaintained | low; text-rendering transitive of egui. Monitor; upgrade rides the egui upgrade lane (this lane must not touch Cargo.lock) |
| zbus_names 4.3.5 | RUSTSEC-2026-0371 | unmaintained | low; Linux D-Bus stack, not on the Windows runtime path |
| zvariant 5.15.1 | RUSTSEC-2026-0370 | unmaintained | low; same Linux stack |
| zvariant_derive 5.15.1 | RUSTSEC-2026-0372 | unmaintained | low; same Linux stack |
| clipboard-win 5.4.1 | yanked (no advisory) | yanked release | low; transitive of the clipboard integration; no vulnerability filed |

**No unresolved high- or medium-severity advisory.** The warnings cannot be
fixed in this lane by design (no dependency upgrades here); they ride the next
integration dependency bump.

## 2. Licence audit and the font-notice finding

`cargo deny check licenses` with `scripts/security/deny.toml` (policy: the
standard permissive OSS set; unlicensed = deny):

- **Finding F-1 (medium, distribution compliance, remedied in-lane):** the
  shipped `aperture.exe` embeds default fonts via `epaint_default_fonts`
  0.37.0 under **OFL-1.1 and Ubuntu Font Licence v1.0** (plus MIT-licensed
  Hack/emoji-icon-font binaries and a Bitstream Vera notice). The engine's
  MIT/Apache licensing does not clear these font licences' notice
  requirements, and no font notices were carried in the repository or the
  package. **Remedy applied here:** the upstream licence texts were copied
  verbatim into `docs/security-audit-notices/` (with a README mapping each
  file to the shipped font). The check now reports `licenses ok`.
  **Residual:** the packaging step should ship or reference these notices in
  the v1 zip — flagged to the packaging owner for the release-time recheck.
- All other 498 crates clear the policy. The clarify/allow list contains only
  licences actually encountered; nothing copyleft (GPL/AGPL) is in the graph.

This is not a claim of legal clearance; it is a complete, evidenced licence
inventory with the notices the bundling requires now carried in-repo.

## 3. Credential audit

Command: `python scripts/security/scan_credentials.py [--history]`.

- High-signal secret patterns (PEM keys, GitHub/AWS/Slack/OpenAI/Anthropic/
  Google token shapes), credential-assignment review patterns, and
  sensitive-filename checks over all tracked files: **0 findings**.
- Full object database (every blob, reachable or not — deleted secrets
  included; 1 MiB bound, binary skip): **0 secret-tier findings**; 13
  review-tier matches, all verified benign — the generic
  credential-assignment regex caught MTG *token*-mechanics identifiers in old
  engine/test revisions (e.g. `let token = object.card.is_token();`), not
  credentials.
- Findings are redacted by construction (≤3-char prefix + length); the scanner
  cannot print a credential value. Boundary: this scanned the repository only,
  never the user's unrelated machine or credential files.

## 4. Untrusted-input mutation harness

New, narrowly named, additive-only harnesses (no production code touched):

- `crates/core/tests/security_mutation_fuzz.rs` — 7 tests
- `crates/import/tests/security_mutation_fuzz.rs` — 7 tests

Method: bounded deterministic mutations (seeded LCG, iteration counts 256–512
per surface, injected vector caps at 100 000 elements, oversized probes at
10 MiB), executed against the real public boundary — `Replay::from_json`,
`SessionSave::from_json`, `Game::submit`/`validate`, `parse_deck_url`,
`import_text`, `text_report`, `normalize` — not a detached JSON toy.

Properties proven (all green):

1. **No panic / no hang** across structural byte-level mutations of replay,
   save, provider-JSON and deck-text inputs.
2. **Canonical URL invariant:** over 512 seeded URL mutations plus
   id-normalization shapes (`+id`, leading-zero, uppercase scheme, default
   port), any accepted parse rebuilds `https://archidekt.com/decks/<id>` from
   the numeric id alone; hostile shapes (`%00`, RTL override, id overflow,
   10 MiB URL) are refused. Existing fixed-case coverage in
   `crates/import/tests/url_guard.rs` (scheme/host/path/SSRF forms) is
   unchanged and still green.
3. **No quantity wrap:** hostile counts (`99999999`, `-3`, `2.5`, `1e3`,
   `0x10`, `256`) are refused or reported exactly — never silently wrapped
   into a legal deck.
4. **Tampered saves/replays are refused:** forged save digest →
   `SaveDigestMismatch`; swapped embedded replay → identity-binding refusal;
   wrong version/seats/rules revision → typed errors; u64::MAX
   revision/decision and nonexistent object ids are rejected at replay/submit.
5. **No state drift:** every rejected mutant submission leaves the engine
   digest bit-identical; `validate()` (action legality) and `submit()`
   (legality + submission identity/staleness) were exercised at 40 real
   decision points × 9 mutant classes.
6. **Bounded hostile sizes:** 100 000-row provider payloads normalize with an
   honest count; 128–100 000-deep nesting hits serde's recursion guard; a
   10 MiB-padded replay either loads (whitespace-inert, digest reproduced) or
   is refused.

**Observation F-2 (low, recorded, accepted):** the local replay/save parse
boundary has no file-size cap (the remote fetch path has the 8 MiB cap).
Local saves are user files and content-bound by digest, so impact is a local
resource spike from a crafted shared file, not corruption; recorded as
residual risk with the whitespace-inertness property pinned by test.

## 5. Distributed content vs. engine licences

Inventory of what is actually distributed (from `scripts/package.ps1` bundle
contents): `aperture.exe`, `aperture-cli.exe`, `decks/*.txt` (5 practice
lists), `BUILD-STAMP.txt`, `README-PACKAGE.md`, `launch.cmd`,
`verify-package.cmd`, `verify-package.ps1`, `MANIFEST.sha256`.

- **Engine code:** original source, MIT OR Apache-2.0 (`LICENSE-MIT`,
  `LICENSE-APACHE`), verified in root manifest.
- **Fonts:** embedded in the exe via egui/epaint — notices now carried in
  `docs/security-audit-notices/` (F-1 above).
- **Card/rule content:** the shipped decks contain real Magic card names and
  counts only — no art, no card text, no rules documents. The repository's
  documented position (`ASSETS.md`) is nominative/factual interoperability
  use by an unofficial project, not endorsed by Wizards of the Coast;
  `ASSETS.md` explicitly does **not** claim relicense of card names/text/art.
- **Test fixtures (not shipped):** the Scryfall oracle subset carries its own
  manifest with source, digests and a Wizards/Scryfall licence note; the
  Archidekt provider snapshot is an integration-test fixture, not a bundled
  card database. `ASSETS.md` correctly gates packaging of any card data on a
  separate third-party content review.
- **Model weights/art:** none bundled; the optional Decider is a separate
  loopback service (`127.0.0.1:8765`, refusal enforced at
  `crates/ai/src/decider.rs:218`), never a bundled model.
- **Docs site:** links only to `api.github.com` and the project's own GitHub
  pages; no third-party asset loading; the board scripts are token-free
  (authentication stays in `gh`'s credential store).

**Explicit unresolved distribution risks (not certified away):**
1. The card-name/nominative-use position is documented, but no formal legal
   clearance exists for card-name distribution; treated per ASSETS.md as the
   release position with the gate kept open for a third-party content review.
2. Font notices exist in-repo; the v1 package must ship or reference them
   (packaging owner, release-time recheck).
3. Incoming deltas — #17 corpus lane (PR #46) and the #25 model lane — change
   distributed content class (certified lists / model adapters) and require a
   versioned recheck of this audit at final packaging.

## 6. Verdict against the release criteria

- Dependency/licence/credential audit with triaged evidence: **done this
  slice** (sections 1–3, commands and versions exact).
- Fuzz/mutation of import/replay/save/URL/action inputs: **done this slice**
  (section 4; 14 new tests, all green; bounded and deterministic).
- Distribution rights distinguished from engine licences: **reviewed and
  documented** (section 5), notices remedied in-lane, residual risks listed.
- "No unresolved high-severity security issue, private-data leak or
  unlicensed bundled asset **at release**": nothing high-severity was found
  at this head; the final statement belongs to the release gate (#33) after
  the #17/#25 deltas land and are rechecked against this audit.
