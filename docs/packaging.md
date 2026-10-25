# Packaging and release artifacts

How a reviewable, reproducible desktop practice build is produced, and exactly
what "reproducible" is claimed to mean here.

This lane owns packaging/release scripts, artifact CI and packaging docs. It does
not own engine rules, the table renderer, import providers or ops/reporting, and
it does not certify the practice corpus: **release sign-off still requires the
independent corpus gate** (issue #17), which this document does not waive.

## One command

```
scripts\package.cmd                 # Windows, from a clean committed tree
```

or, identically:

```
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package.ps1
```

Output (both `dist/` paths are git-ignored):

```
dist/the-aperture-<version>-<shortsha>/
    aperture.exe
    aperture-cli.exe
    BUILD-STAMP.txt
    MANIFEST.sha256
    README-PACKAGE.md
    launch.cmd
    verify-package.cmd
    verify-package.ps1
    decks/*.txt
dist/the-aperture-<version>-<shortsha>.zip
```

The script:

1. Reads the workspace version from `Cargo.toml`, the full commit SHA and commit
   date from git, and the rustc/cargo versions and host triple.
2. **Refuses a dirty working tree** unless `-AllowDirty` is passed. A stamp that
   names a commit which does not contain the shipped sources would be a lie, so
   the default is refusal; `-AllowDirty` marks the package `-dirty` and records
   `source_dirty=true` in the stamp.
3. Builds `cargo build --release --locked -p aperture-ui -p aperture-cli`.
4. Assembles a fresh bundle directory (previous output for the same name is
   deleted, so no stale file can survive into the manifest).
5. Normalises text payloads while copying: `decks/*.txt`, `README-PACKAGE.md`,
   `BUILD-STAMP.txt` and `MANIFEST.sha256` are written with LF; `launch.cmd`,
   `verify-package.cmd` and `verify-package.ps1` with CRLF. The bundle therefore
   does not depend on the checkout's line-ending configuration.
6. Writes `MANIFEST.sha256`: `sha256␠␠relative/path`, sorted ordinal by path,
   LF-terminated, UTF-8 without BOM — the same format `sha256sum -c` reads.
7. Writes the zip with every entry timestamp pinned to `2000-01-01T00:00:00Z`
   and a fixed entry order.
8. Runs the packaged verifier against the result and fails the run if it fails.

Flags: `-Version`, `-OutDir`, `-AllowDirty`, `-SkipBuild`, `-SkipVerify`,
`-SkipSmoke`, `-NoZip`.

## One verification command

From inside an unpacked package:

```
verify-package.cmd
```

(`powershell -NoProfile -ExecutionPolicy Bypass -File verify-package.ps1`)

It exits `0` only if all of the following hold:

* every file listed in `MANIFEST.sha256` exists and re-hashes to the recorded
  SHA-256;
* no payload file exists that the manifest does not list (added or renamed files
  fail the check; the verifier's own transient smoke PNG is exempt);
* `BUILD-STAMP.txt` has `format`, `version`, `commit`, `target`, `rustc`,
  `scope`, records `signed=false`, and its `commit` is a full 40-hex SHA;
* `aperture-cli.exe --version` reports the stamped version, tying the binary to
  the stamp;
* `aperture.exe --smoke <temp>.png` launches the packaged desktop executable,
  writes a PNG of plausible size with a real PNG signature, and exits `0`.

Any problem is printed with the offending path and the command exits non-zero.
`-SkipSmoke` is available for environments without a desktop session; the
default is to prove the launch, because that is the claim that matters.

Manual equivalent for the hashes:

```
sha256sum -c MANIFEST.sha256
```

## Version / commit stamp

`BUILD-STAMP.txt` is deterministic for a given commit and toolchain — it
deliberately contains **no build timestamp**, so rebuilding the same commit does
not change the bundle's hash set:

```
format=1
name=the-aperture
version=0.1.0
commit=<40-hex>
commit_short=<7-hex>
commit_date=<ISO-8601 committer date>
source_dirty=false
profile=release
target=x86_64-pc-windows-msvc
rustc=rustc 1.97.1 (...)
cargo=cargo 1.97.1 (...)
rustflags=
scope=bounded-28-card-practice
signed=false
```

`rustflags` records an inherited `RUSTFLAGS` value (normally empty; the
reproducible-link flag lives in `.cargo/config.toml` instead, so it is part of
the repository and shows up as a normal reviewed diff).

## Reproducibility contract

**Inputs.** The stamped commit plus `Cargo.lock` plus the pinned toolchain plus
`.cargo/config.toml`. Nothing else: no environment variables, no network at
runtime, no generated code outside the repository.

**Contents and hashes.** Deterministic. Two runs of `scripts/package.ps1` on the
same commit produce the same file set, the same `MANIFEST.sha256` bytes and the
same zip entry order/timestamps. Text normalisation (above) is what makes this
independent of checkout line endings.

**Binaries.** `.cargo/config.toml` adds MSVC `/Brepro`, which drops the
per-invocation PE timestamp and derives the debug-record id from content.
Without it, two builds of identical sources differed by exactly ~20 metadata
bytes in the PE header/debug directory; with it, three independent clean rebuilds
of the same commit and toolchain produced byte-identical executables
(`aperture.exe f9fd972d…`, `aperture-cli.exe 75da8e9d…`). The CI package job
re-proves this on every pull request by rebuilding from `cargo clean` and
comparing the shipped hashes.

**Not claimed.**

* Bit-identical binaries across different hosts, compilers, toolchain versions
  or operating systems. Absolute dependency paths and compiler internals differ;
  only same-commit/same-toolchain/same-host builds are proven identical here.
* Bit-identical zips across different .NET runtimes (the DEFLATE implementation
  is runtime-provided). The *unpacked* bundle is the reproducible artifact; the
  zip is a transport convenience and its hash is printed, not promised.
* Any code signing. The build is unsigned; `signed=false` is recorded and the
  packaged README says so.

### Measured cross-host check (not a claim, just evidence)

Commit `8d65f74` was packaged locally (rustc 1.97.1) and by the CI `package` job
on `windows-latest` (runner stable, rustc 1.99.1). Of the 12 hashed files, 8 were
byte-identical across the two hosts: `README-PACKAGE.md`, all five deck lists,
`launch.cmd`, `verify-package.cmd` and `verify-package.ps1`. The other four were
the two executables and `BUILD-STAMP.txt`, and the stamp difference was exactly
its `rustc=`/`cargo=` lines. The zip hashes differed too (different .NET runtime
for DEFLATE). That is the contract above behaving as advertised: source-derived
contents are host-independent, compiled artifacts are tied to the recorded
toolchain, and the stamp never hides a toolchain change. The CI job proves
same-toolchain byte-identity within its own run by rebuilding from `cargo clean`
and comparing against the shipped hashes.

## CI

`.github/workflows/ci.yml` gains a `package` job (windows-latest):

1. builds the package with `scripts/package.ps1` (which builds, assembles,
   manifests and verifies, including the packaged desktop launch);
2. runs `cargo clean` and a second release build, then fails if either shipped
   executable hash differs — the binary-reproducibility proof above, on CI;
3. prints the manifest and uploads `dist/*.zip` with the pinned
   `actions/upload-artifact` commit SHA and `contents: read` permissions.

Existing `rust` (three-OS tests/fmt/clippy) and `board` jobs are untouched.

## Honest limitations

* Unsigned Windows build; no installer, no auto-update, no signing certificate.
* Bounded 28-card practice pool, 2–4 seats, `ordinary` (20 life) and bounded
  40-life practice profiles — **not** Commander, not full Magic.
* The desktop table needs a real window; the packaged smoke mode exercises it but
  is not a substitute for a human session.
* Packaging proves the artifact is intact and reproducible. It does not prove the
  rules engine is correct — that is what the test suite and the independent
  review/corpus gates are for.
