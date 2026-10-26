# Bundled font licence notices (security audit lane, #31)

The shipped `aperture.exe` links `egui`/`epaint`, which embed default fonts
through `epaint_default_fonts` v0.37.0. The dependency graph therefore carries
four font licences beyond the workspace's MIT OR Apache-2.0 engine licence.
`cargo deny --config scripts/security/deny.toml check licenses` passes only
with these accounted for.

This directory carries the upstream licence texts **verbatim** so the bundled
redistribution satisfies each font licence's notice requirement:

| File | Applies to | Licence |
| --- | --- | --- |
| `OFL-1.1.txt` | `NotoEmoji-Regular.ttf` | SIL Open Font License 1.1 |
| `Ubuntu-Font-Licence-1.0.txt` | `Ubuntu-Light.ttf` | Ubuntu Font Licence v1.0 |
| `Hack-font-notices.txt` | `Hack-Regular.ttf` | MIT (Source Foundry) + public-domain DejaVu + Bitstream Vera with reserved names |
| `emoji-icon-font-MIT-license.txt` | `emoji-icon-font.ttf` | MIT |
| `egui-icons-notices.txt` | `egui-icons.ttf` | MIT subset of emoji-icon-font |

Source of the texts (copied without modification):
`epaint_default_fonts-0.37.0/fonts/*.txt` in the crates.io package, upstream
repository `emilk/egui` (crate `epaint_default_fonts`).

Release note for the integration/packaging owner: the final v1 package should
either include these notice files alongside `README-PACKAGE.md` or reference
this directory, so the distribution carries the font notices it is required to
carry. No card art, card database or model weights are bundled; see
`ASSETS.md` for the separate card/rules/art/model rights posture.
