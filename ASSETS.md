# Source and third-party content

## Original project source

THE APERTURE's original source is dual-licensed **MIT OR Apache-2.0**, matching the workspace metadata. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

## Separate rights and provenance

The source license does **not** relicense Magic card names/text/art, official rules documents, third-party engine source, deck-provider responses, dependencies or model weights. Preserve applicable notices and track provenance before distributing those materials. This is an independent, unofficial project, not endorsed by Wizards of the Coast or Hasbro. Relevant names and marks belong to their respective owners.

The existing provider JSON fixture is an integration test snapshot, not a bundled card database or a claim of content ownership. Do not expand fixtures into a scraped art/text collection.

The Commander legality fixtures under
`crates/import/tests/fixtures/commander/real/` are a small selected subset of
Scryfall oracle card objects (names, type lines, Oracle text, color identity and
Commander legality only; no images or prices). `manifest.json` records the bulk
source URL, capture time, per-line digests, the full bulk digest and a
Wizards/Scryfall license note. They are interoperability test fixtures, not a
redistributable card database. Packaging requires a separate third-party content review and an explicit support manifest.

The working-tree candidate references the official Gatherer page for **Crack the Earth** (<https://gatherer.wizards.com/BOK/en-us/98/crack-the-earth>) to record its name, `{R}` cost, Sorcery — Arcane type and one-line Oracle text. The captured source file lives in the run scratch directory, not the repository; its provenance hash is recorded in [docs/rules-reference.md](docs/rules-reference.md). No card art or full card database is bundled, and the bundled practice decks are unchanged.

## Release policy

* Prefer original graphical surfaces and user-supplied or separately permitted assets.
* Do not bundle proprietary client assets or copy engine source under incompatible terms.
* Do not bundle downloaded model weights merely because the adapter is FOSS; model and dataset licenses remain separate.
* Dependency notices, official rules provenance and any distributable card data/art must be accounted for before v1. Until that review is complete, the release gate remains open.
* The optional Decider service is separate from the application and is not required for normal offline practice.
