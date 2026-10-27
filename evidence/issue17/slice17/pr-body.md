## Partial starter corpus — issue #17 (draft, not acceptance)

Eight desired100-card Commander snapshots are published. Only `jasmine-vanilla-stompy` is certified end to end; seven remain planned/unsupported. This is an increment toward the original eight-list gate, NOT full Commander or v1.

### Current state

- Catalog **134 real cards**.
- Desired eight-list union **459 names = 122 supported + 337 missing**.
- **1 certified / 7 planned**; desired snapshots unchanged, no bundled replacements.
- Nearest list, Ayula: **31 missing names**, including its commander interaction.

### Latest increment: slice17

Three sourced Ayula-list cards with immutable printing URLs/retrieval stamps in `fixtures/starter-corpus/slice17-sourced.json`:
- **Friendly Teddy**: 2/2 artifact creature; dies → each player draws. Serialized all-player draw progress preserves later seats through commander replacement and supports mid-instruction save/load.
- **Ulvenwald Bear**: morbid enters trigger, two +1/+1 counters on target creature, either controller; token deaths count.
- **Caller of the Claw**: flash, enters → Bear per nontoken creature put into its controller's graveyard from battlefield this turn, using ownership rather than battlefield control.

Death history survives repeated cleanup and resets only when the turn actually advances. Unaffected closed-corpus replay digests retain their old encoding. AI-file delta only initializes/copies the new public state fields; no policy/search change.

### Verification and review

- True original-history practical adjudication **APPROVED_INCREMENT** at `3934d76`, with final slice17 6 / command-zone34 / replay6 and separate replay/save checks. Parent corrected a prior launch-path error that had created a new review history at `bfaa782`; both histories/results remain preserved and the false continuity claim is explicitly corrected. No product source changed after `bfaa782`; later commits only strengthen tests or record evidence.
- Parent final focused suites: slice17 **6**, printings **8**, catalog **5**, slice15 corpus **2**, all pass; fmt check passes. Reviewer separately ran lib12/command-zone34/slice17-at-review5/replay6/ordering2. Final workspace clippy warnings-denied passes; releaseCLI built. Required CI is checked against the exact final publication SHA, not inherited from a reviewed sibling.
- One real **4seat/40life/40-card package** journey, exact `decks/package-slice17.txt`, seed1616: terminalP3, **1866 actions / 42 turns / 4952 events**, digest `85fabffc9aa1…`; separate releaseCLI replay and fresh-session-save replay match. Teddy12/Ulvenwald8/Caller8 casts;4 Bears created.
- Driver is evidence-only, masked/offered-action based, NOT productAI or strength evidence. It injects **Jasmine Boreal in every command zone**. Package is not another desired100-card starter certificate.
- Two first-cut defects (cleanup reset and losing later players' suspended draws) repaired before approval. Initial scaffold-first/TDD and permissive-assertion limitations are disclosed, not retroactively relabelled.

Current checkpoint: `evidence/issue17/SLICE17-HUMAN-CHECKPOINT.md`; review: `evidence/issue17/slice17/REVIEW17.md`.

### Exclusions / original work remains

Equipment, Auras, soulbond, hideaway, mounts/saddle, Ring temptation, regeneration, tutors, reanimation, X-spells, multi-mode text and most tribal payoffs remain outside the supported corpus as recorded per list. Arbitrary Archidekt compatibility remains an explicit non-goal. No eight-list acceptance, merge, issue closure or certified-evaluation/soak unlock is requested.
