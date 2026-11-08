# Native testing: local AI, clipboard and document imports

## Run outside the restricted command runner

The November 8 native-test log records Qwen3-8B loaded and listening after about 13 seconds. The restricted command runner denies the application's loopback connection with Windows socket error 10013. The old startup loop swallowed this error and eventually reported a false two-minute model-load timeout. Access-denied errors now stop immediately, preserve the draft, and show the actual connection error and log path. Increasing the loading timeout does not resolve an access restriction.

Build the embedded UI and Windows executable with `npm.cmd run build:native`, then open `Start-Dungeon-World-AI.cmd` from File Explorer in the user's normal Windows session. No development web server is needed. The launcher reuses the existing local llama.cpp and Qwen GGUF, or respects `LOCAL_LLM_SERVER_PATH` and `LOCAL_LLM_MODEL_PATH`. Nothing is downloaded during launch. The current development runtime/model still live in separate folders; this is not a qualified self-contained installer.

The build respects `CARGO_TARGET_DIR`. Its native output is copied to the ignored `build/native` folder. An explicit `DWA_DATA_DIR` at build time is retained in that folder's local `launch-settings.json`, so an isolated test campaign can be reopened from the launcher. Otherwise the app uses its normal app-data folder. `node scripts/native-dm.mjs --data <folder>` provides an explicit session override.

If Windows locks the executable because the app is open, the build stages a versioned executable and records its basename in the local launch settings. Close the old app and use the same launcher to open the update. The launcher accepts only the expected application basename pattern from those settings; explicit `DWA_NATIVE_APP_PATH` remains an environment override.

## Clipboard

Windows text fields use native plain-text clipboard commands for Ctrl+C, Ctrl+X and Ctrl+V. The Dungeon Master action box and pasted-source form also offer Paste buttons. Reads occur only after an explicit paste gesture; there is no clipboard polling. Paste replaces the current selection, retains surrounding text, updates the draft, and respects the field's length limit. Password fields retain the platform's normal behavior. Clipboard access failures are displayed instead of silently dropping text. Other platforms retain their native shortcuts; explicit Paste buttons use the browser Clipboard API.

## Compendium

The separate top-level DM library tab is now inside **Compendium → Campaign documents & DM library**. **Reference entries** retains the existing structured spell/monster/item/class/species/background collections. Both kinds of saved material are available through the DM's local reference tools, subject to the campaign's edition rules.

The document importer accepts PDF, Word DOCX, TXT, Markdown, CSV, HTML and arbitrary JSON, as well as existing campaign source packs and supported dnd-data arrays. A file's name no longer has to match one of six collection filenames merely to be accepted as reference text. Recognized dnd-data arrays retain their structured collection format. Other JSON is indexed as labeled source text.

PDF preparation first tries the installed Windows PDF text filter. If that is unavailable or yields no text, it tries offline Windows PDF rendering and OCR. OCR availability depends on installed Windows components/languages and the application's deployment context. Failed or unreadable files produce an error. The review identifies OCR, missing text where detectable, and layout limitations; it does not assert that every illustration or scanned page was interpreted. Review names, numbers and tables before adding a source. Older Word `.doc` files need a DOCX export.

DOCX extraction reads bounded stored/deflated Word XML, paragraphs, table cells, headers/footers and footnotes/endnotes. It validates archive lengths and checksums, rejects document types/external entities and unsupported encryption, and never unpacks arbitrary paths. HTML imports are stripped as text without creating an active browser document or loading linked resources.

Choose files, review/edit their extracted text and source metadata, then select **Add reviewed sources to Compendium**. Originals remain in their existing file locations; the campaign stores the indexed text and provenance. Limits are 75 MB per input file, 8 MB of UTF-8 text per source and 200 prepared sources at a time. Previously saved items are removed from a partially failed batch, allowing the remaining imports to be retried without duplicating successful ones.

## Verification

Engine tests cover Word archive extraction and invalid inputs alongside existing search, edition, isolation and persistence tests. Frontend tests exercise generic files, source-pack metadata/notices, safe HTML conversion, partial-import retry, document review, clipboard selection/length handling and the Compendium sections. Native tests render/read an actual PDF fixture with Windows OCR and exercise the PDF text-filter/OCR route. A successful extraction test is separate from an end-to-end GUI file import or a real local-model reply.

Before the v0.4 follow-up below, November 8 verification recorded 61 engine tests, 6 AI unit tests, 34 frontend tests and 2 Windows PDF tests passed (103 total). The separate local-TCP integration test remains opt-in and blocked by the restricted runner. Type checking, the offline source check and the embedded Windows build passed.

In the rebuilt native UI, an isolated Mournwell source was saved, indexed and returned in Compendium search. The native clipboard copy command returned Windows Access Denied (error 5); the file chooser did not remain open in this restricted session. Thus host clipboard/file-picker behavior is not certified by those checks. Full live-model qualification still requires the normal-host launcher test, followed by continuity, agency, secret handling, arithmetic/read-tool and installer checks.

## Lookup recovery and continuity (v0.4)

A subsequent normal-host user screenshot confirms two saved local replies and the seven campaign-source imports. It also exposed a real response-protocol issue: the model sometimes drafted narration while requesting a lookup, and the controller rejected the entire turn.

The controller now discards that intermediate prose/choices, performs the validated reads and asks for the completed reply. Only the completed reply can reach the existing save transaction. The response schema offers mutually exclusive final-reply and read-request forms, and permits no further reads after the three-round budget. Limits and rejection of unsupported/write requests remain in effect.

Saved player/DM turns are now sent as alternating chat roles, with the current state and new action in the final user message. This distinguishes the latest action from previous opening narration; it is not a guarantee of model-quality continuity.

Four new regression tests pass: mixed-draft recovery with real engine arithmetic and save/reopen, disjoint response forms, bounded/excessive reads and conversation ordering/trimmed history. The AI suite has 10 passing tests plus the opt-in socket test; with the unchanged prior engine/frontend/PDF results, the recorded total is 107 passing tests. The new controller tests use scripted model responses, not a claim of a new real-model playthrough.
