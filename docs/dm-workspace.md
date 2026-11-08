# Dungeon Master workspace: sources, table, assets and history

The [native import and clipboard guide](native-imports-and-clipboard.md) describes the current embedded Windows launcher and document workflow.

## Local runtime versus a complete package

The development app talks to an app-owned llama.cpp process on IPv4 loopback. That process loads a GGUF from disk. `dev:dm` currently discovers the existing workbench runtime and Hugging Face cache unless explicit local file overrides are set. The development UI is also served locally by Vite. None of these imply a cloud model call.

These are separate development locations, not a self-contained distribution directory. The shipping resource defaults are `ai/llama-server` (Windows: `.exe`) and `ai/models/dungeon-master.gguf` relative to Tauri's resource directory. They are not currently filled by a qualified installer. The complete offline package still needs matching runtime libraries, model, notices/checksums and offline prerequisites, followed by a clean-machine test. GitHub stores application source; it is not needed to play an installed offline package.

## What the DM can inspect

The engine supplies active-scene grid dimensions/scale, zero-based token positions, one-based cell labels, sizes, IDs, linked character IDs, HP/state/light values, and explicit hidden/fog flags. It supplies character values and computed base ability modifiers and level-based proficiency. This is structured table awareness; the text model does not visually interpret PNG backgrounds or detect walls/terrain from artwork.

Registered read tools can search/read local campaign/reference text and the bundled SRD, inspect another saved scene or a token, read character sections, compute distances, and perform bounded arithmetic. They accept typed arguments, campaign IDs and revisions, rather than arbitrary code or queries. There are at most three read rounds with up to two requests per round. Token counting and bounded passages keep the context finite. This supplies access on demand, not every book in every prompt.

GM-only notes, private cards, hidden tokens and source passages are labeled separately. The model is instructed not to reveal them before discovery. Typed tools prevent state writes; secrecy in generated prose still requires model qualification and is not guaranteed by JSON validation.

Distance results describe center-to-center straight-line and one-cell-per-diagonal grid metrics. They do not establish line of sight, cover, reach, movement cost, or a legal attack. Character training/features are not inferred merely from class and level. A die total without check parameters is not a resolved outcome; AC is a total statistic, not a bonus.

## Local story/reference library

Use **Compendium → Campaign documents & DM library** to add PDF, Word DOCX, TXT, Markdown, CSV, HTML, general JSON or a version-1 source pack. Review the extracted text and metadata, then add the sources. PDF preparation uses the installed text filter or offline Windows OCR when available. Structured dnd-data arrays remain under **Reference entries**. Sources retain title, edition, provenance, GM visibility and passage order. The app indexes passages locally with SQLite FTS5. Existing compendium descriptions are backfilled once, and new entries are indexed transactionally. The bundled SRD 5.2.1 is searchable for manual/revised profiles. Explicitly mismatched 2014/2024/2e material is excluded from a selected 5e edition; unassigned/homebrew sources remain labeled for a table ruling.

The source-pack importer displays any `notIndexed` notices, including scans awaiting OCR. Importing a source does not make its adventure events campaign history or create an active character sheet. Source links are provenance labels; no downloads are performed during play. Illustrations/layout and scan OCR require separate preparation.

Current content inventory:

| Material | Readiness |
| --- | --- |
| SRD 5.2.1 | Bundled offline text and PDF; native DM lookup implemented |
| Imported dnd-data collections | Campaign-local full-text search; publisher/book/edition identity retained |
| Aerathis master outline, Levels 1–20 | Current source extracted into a private import pack |
| El-Urukal / Hidden City of Enki | Extracted into the same pack |
| Both Plague of Minauros guides | Full extracted text in the pack, labeled 2014 rules |
| Tal'Dorei material (`TDCampfull.pdf`) | Full extracted text in the pack as optional reference; edition unassigned |
| Laedron and Liswynn Auraward sheets | Extracted as reference sources, not installed as active actors |
| Call of the Netherdeep | Existing scanned PDF needs OCR; not presented as indexed text |
| Storm's Gambit / Merkaba / other past named guides | Source identities remain on the intake list; current files were not resolved in this project's inventory |

Previously collected source locations remain [dnd-data](https://github.com/ThisFriendJosh/dnd-data), [Fari community](https://github.com/ThisFriendJosh/fari-community), the [official SRD](https://www.dndbeyond.com/srd), and the previously supplied [5e book folder](https://github.com/EllatharTheHalfling/DnD-Books/tree/master/5e). These are source/intake pointers, not runtime dependencies or a statement that their entire contents have been imported. Private books and campaign packs stay outside the public application tree.

## Generated visual assets

Open **Art & codex → Token workshop**. Choose a frame, color, symbol, and short label; preview; then save PNG + SVG. The app creates a transparent 512×512 PNG on the device and stores it, a validated SVG, and its recipe in the campaign database in one transaction. Open/export a saved asset to download either format, or place it as a token and save its position with the scene. Original recipes are reusable/editable; arbitrary imported SVG markup is not executed.

This is a procedural token/sprite foundation. Painted maps, portraits, and scene illustration need an additional local image-generation backend. No cloud art service is silently used.

## History edit mode

Choose **Edit history**, select one entry with ×, review its label/time and the explanation, then **Remove entry**. Removal excludes it from the visible timeline and DM conversation/roll context. Current character and scene state is preserved. Later replies can still mention an earlier removed turn; remove those separately if needed.

Choose **Show removed entries** to restore one through the same deliberate confirmation. Original canonical events and edit records are retained for recovery. This is reversible history cleanup, not physical erasure or a replay/undo of all state changes. Edit records themselves cannot be removed. Revision and campaign checks prevent stale or cross-campaign edits.

## Verification boundary

The engine tests cover source/edition/campaign isolation, old-index backfill, import rollback, grid distance, bounded arithmetic, reversible history, SVG escaping, asset atomicity and Word extraction. Frontend tests cover document preparation/review, partial import retries, safe HTML text, clipboard selections, Compendium sections and existing escaped views. Native Windows tests successfully read a real PDF fixture with OCR. The rebuilt GUI saved/indexed/searched a test source. Native clipboard copy returned Windows Access Denied (error 5), and the file chooser did not remain open in this restricted session. The local AI connection remains denied with socket error 10013. These restrictions require a normal-host launch before clipboard, file-picker and live-model behavior can be qualified; token rendering also retains its native acceptance check.

Verification in this working tree: 61 engine tests, 6 AI unit tests, 34 frontend tests and 2 Windows PDF tests pass (103 total). The separate socket integration test is ignored by default and remains blocked here. Type checking, the offline source policy and the embedded native build pass. A prior isolated import of the seven-source pack stored 730 passages, reopened successfully and returned local searches for Mournwell, Enki, Ilmater and Exandria. This is extraction/retrieval/persistence evidence; live model quality and installer qualification remain separate gates.

The supplied source pack can be qualified without a model or a live campaign:

```powershell
cargo run --offline --manifest-path services/engine/Cargo.toml --example import_source_pack -- path/to/source-pack.json path/to/isolated-test-output
```

Keep [beta feedback priorities](beta-feedback-roadmap.md) and [offline packaging gates](offline-dm-design.md) alongside subsequent changes.
