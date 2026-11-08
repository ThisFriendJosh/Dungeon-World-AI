# Beta feedback and current priorities

The current user-requested tab priority is: **1. Game Timers & Calculators; 2. Scrying Pane (CEF browser); 3. 3D Dice animations.** This build adds those tabs in that order. See [game tools, browser and dice](game-tools-scrying-dice.md) for behavior and acceptance checks.

Two completed beta1 surveys were reviewed. Both reported successful campaign/character/map/art/dice/reference/save/reopen tasks, responsive native UI, no crashes or lost saves, and 5/5 overall ratings. These are short beta observations, not broad reliability or AI-quality evidence. One tester needed help launching. AI was unavailable or uncertain in those surveys; a subsequent user screenshot shows one real local reply saved successfully.

| Requested improvement | Current response | Remaining acceptance criteria |
| --- | --- | --- |
| AI narration and story memory | Local narrator, persistent turns, canonical campaign context, local reference search and read tools | Run the new tool loop on the host; held-out agency, continuity, secret handling and missing-stat scenes; durable summaries and fact retrieval |
| Campaign management | Per-campaign story/reference imports and source provenance; deliberate history editing | Campaign backup/export/import and richer campaign setup |
| Save reliability and recovery | Transactional turns/assets/imports/history edits; duplicate/stale protection; event-log repair | Kill/restart and migration tests on copies of real campaigns; clean-machine tests |
| Rules uploads and JSON import help | Compendium document library with PDF/Word conversion, text/Markdown/CSV/HTML/general JSON, source-pack review and existing dnd-data imports | Normal-host file-picker test, OCR quality/language checks, source/edition corrections, duplicate-source/version management |
| Combat and initiative | Read-only grid/character inspection and engine math; current manual tabletop | Persisted encounter order, rounds, actions/reactions/resources, reviewed checks/attacks/damage adapters |
| Group / multiplayer | Scope retained as a separate milestone | Local-first session host, player views, permissions and reconnect behavior before an optional remote mode |
| Rules / codex / data | Full-text indexing of imported descriptions plus source-tagged passages and bundled SRD lookup | Better retrieval evaluation and wider reviewed mechanics coverage |
| Visual assets | Local AI sprite workshop with PNG/SVG recipes, image uploads, categories and custom tags; real GPU generation verified | Broader art-style evaluation, saved map/scene generation recipes, and clean-machine setup checks |
| Offline out-of-box distribution | Local inference adapter and explicit resource defaults | Pinned runtime/model/licenses, complete package and offline prerequisites, no-first-download clean install |

The next build must preserve all beta1 save formats. Public source-control changes contain application code and anonymized priorities. Model weights, campaign databases, private reference books, survey exports, and source-pack text are not application source files.
