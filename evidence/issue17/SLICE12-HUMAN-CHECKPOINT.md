# Issue17 — slice12 reviewed increment / slice13 live continuation

Updated2026-10-26T18:17Z. Full original#17 is UNFINISHED. Same card
`t_67778feb`, branch `aperture/issue-17`, draft PR46. No merge, master push,
forcepush, issueclosure or certified downstream unlock.

## Delivered, independently adjudicated code

Head `985d8ffe115536068606a851f8b0f34676f17898` adds three sourced playable
artifacts: Sol Ring, Mind Stone, Hedron Archive. Real hand→stack→battlefield
casting, immediate tap mana, and stacked tap+sacrifice+generic-mana draw
abilities. Explicit mana activation is required: default product heuristic
currently ignores artifacts; no AI-policy edit was made. Commander's Sphere
remains excluded because commander-color mana is not modeled.

All6source fields independently matched the3immutable Scryfall printingids
(`slice12/native-source-check.json`). Catalog119;459union=107supported+
352missing. Exactly1of8desired starters certified; desired deck snapshots
unchanged. No new 100card certificate.

## Actual execution

Evidence-only masked artifact-preferring legal-action driver, not productAI or
strength evidence: `slice12/journey-driver`. It reads exact40card
`decks/package-slice12.txt` into all4seats, Krenko commandereach,40life,seed1201.
TerminalP3,1573accepted actions/32turns,6981replayevents,
digest `d3eade58f133c278114cb1289b5ac42e1be51999b4179faca972ec9800fac0ec`.
Actual casts SolRing7/MindStone8/HedronArchive10; manaactivations53/7/6;
stacked drawactivations8/9. Separate releaseCLI replay and freshsave→reload→
replay all match. Original defaultheuristic game retained unchanged:
1746actions/7051events,digest328dadbdbdc6; zeroartifactcasts, not evidence of
new behavior. Both exactpackage snapshots are40cards, NOT desired100cardstarters.

## Real failures retained and repaired

1. Initialfd18dfc directbattlefieldtests masked uncastable artifacts. Native
inspection caught creature-only spell predicates. Real hand-cast RED, then
7266ee5 canonicalartifact permanent repair.
2. Review1found freshsaves rejected departedplayerGoblin archives in internal
Removedstorage. Originalwriter 'save replay matches' claim wasfalse and is
explicitly superseded. Strict validation now recognizes ONLY inert archives of
departedowners, while live/off-BF tokens remain rejected.
3. Review2found archive token references still accepted in trigger_queue and
revealed. Typed `object_is_referenced` now covers actuallive references. Same
independent probe rejects them; ordinary tokencessation and sourceleft behavior
unchanged. No digest/version reset. Oldslice8replay still matches.
4. RequiredCI caught import/UI TESTfixtures using newlysupportedSolRing as
unsupported sentinel. Testonlysentinel refreshed to DarksteelIngot, strong
refusal/singleton assertions retained. No productionimport/UI edits.

Final samehistory review3 `slice12/REVIEW12-R3.{md,json}` APPROVED_INCREMENT,
0remainingblockingfindings. Actualfinalcontrols11core+1UIviewport+22UIsession;
earlier affected artifact/death/token/commander/printing/import21suites reused.
fmt/Clippydenywarnings/releaseCLIbuild passed. Exact985d8ffCI stillRUNNING at
this checkpoint; `slice12/exact-head-ci.json` is a pendingreceipt, NOT green.
Require fresh finalheadchecks before integration.

## Live continuation / recovery

Sixslice12assignments settled. Authorizedrolling slice13 assignment1 was
finalrepairadjudication; assignment2 is SAME sole writer on sourcedgreen
landfall/tokenpackage Sazh'sChocobo/RampagingBaloths/ProfessorofZoomancy.
Assignment `slice13/ASSIGN-WRITER13.md`; proc_9c07751f76f9,22minhardcap,
launched18:15Z. Nativeowner must consume exit+inspect, then SAMEreviewer after
writerstops. Candidate352/352missingcardtexts were fetched from Scryfall in
`slice13/candidate-source.json` but that bulkfile is local research, not a new
supportedcatalog. Only actuallyimplemented cardrecords may bepublished.

Originalsaved histories (both backedup in slice12-backup/slice13-backup):
- writer `evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl`
- reviewer `evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl`
Runoverlay `.starter-run.yml` preserves globalroutes and disables paidfallback.
Existingcontinuoussupervisor owns samecardrenewal; no human/quota block or new
team is required. CurrentworkingCIstate must be reread, never inherited from
oldercodeheadchecks. #32child remainsunassignedTODO,untouched.
