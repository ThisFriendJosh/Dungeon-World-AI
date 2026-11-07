# Repository review and integration plan

Reviewed 2026-11-07 against the connected ThisFriendJosh GitHub account.

## Useful repositories

| Repository | Evidence from the repository | Role in Dungeon-World-AI | Integration status |
| --- | --- | --- | --- |
| [Dungeon-World-AI](https://github.com/ThisFriendJosh/Dungeon-World-AI) | Tauri 2 desktop shell, Vite/TypeScript UI, Rust/SQLite persistence, local AI interfaces | Main application repository | Campaign workspace implemented in this change |
| [fari-app](https://github.com/ThisFriendJosh/fari-app) | React 18 VTT, sheets, dice, drawing; AGPL license; Liveblocks and Sentry dependencies; Node 16 engine declaration | Source for a future local tabletop and sheet adapter | Scene/index-card adapter implemented; native dice tray and drawing canvas added |
| [gamemasterai](https://github.com/ThisFriendJosh/gamemasterai) | AI DM/notetaker design; Express/Vue/MongoDB/OpenAI stack; actual LICENSE is MIT, while package.json says UNLICENSED | Reference for narration and campaign memory modules behind the existing proposal interface | Review completed; local adaptation pending |
| [tauri](https://github.com/ThisFriendJosh/tauri) | Cross-platform Rust/webview desktop framework | Desktop packaging foundation, consumed through published dependencies | Already used by the application |
| [dnd-data](https://github.com/ThisFriendJosh/dnd-data) | JSON backgrounds, classes, items, monsters, species, spells; source publisher/book metadata; README lists 2014/2024 and third-party books | Candidate for a separate, versioned compendium importer | Local source-tagged JSON importer implemented; wider data stays local |
| [BitNet](https://github.com/ThisFriendJosh/BitNet) | Specialized inference runtime for supported low-bit models | Optional local model backend after the engine proposal boundary works | Hardware/model evaluation pending |
| [whisper](https://github.com/ThisFriendJosh/whisper) | Local speech recognition, Python/PyTorch and FFmpeg dependencies | Optional voice input and session transcription | Later milestone |
| [fari-community](https://github.com/ThisFriendJosh/fari-community) | Community resource website; code AGPL; individual resources have their own licenses | Reference for supported resource and sheet formats | Per-resource review needed |
| [fari-games](https://github.com/ThisFriendJosh/fari-games) | README identifies it as the old website redirecting to fari.community | Historical reference | Retired website |

The dnd-data repository's MIT label describes its repository license. Its publisher/book fields still need review for the specific content distributed in any application pack. A pack manifest should retain source, edition, attribution, and distribution terms for each included dataset.

GameMasterAI's actual MIT LICENSE must accompany any adapted code. Resolve its package metadata mismatch when importing a snapshot. Fari-derived application code remains under this project's AGPL-3.0-or-later license.

## Current milestone: a usable saved campaign

Implemented:

- Campaign picker with last-save times and distinct folders for names that normalize to the same slug.
- Character creation and an editor for level, ability scores, HP, armor class, inventory, conditions, and notes.
- Explicit saves, error feedback, unsaved-change prompts when navigating within the app, and a recent activity list.
- A standalone Rust persistence library, used directly by Tauri.
- Character and event writes in one SQLite transaction.
- A rebuildable JSONL mirror with a visible warning if a database save succeeds but the mirror refresh fails.
- A Node-based offline source guard and platform-specific desktop bundle settings.

Sheet values are manually recorded. Character creation does not calculate class features, hit dice, proficiencies, equipment effects, or spells. Inventory and conditions support text entries; existing structured entries are preserved when their displayed text is left untouched.

## Following milestones

1. **Local tabletop adapter.** Scene/index-card adaptation is implemented in the Tabletop view. A native dice tray with engine-generated rolls is now added. The oak canvas adds five-foot maps, saved scene selection, local image import, token state, drawing, fog, and circular lighting behind the campaign API. 62 supplied source images are reviewed and bundled as maps, scenes, portraits, atlas entries, GM references, and table styling. Audit Liveblocks, Sentry, external images, translation loading, and environment access. Keep the desktop port/output contract.
2. **Local DM proposal loop.** Adapt GameMasterAI's useful DM/notetaker patterns behind services/ai/src. Feed canonical events and an explicit summary into a local model; present proposals for validation before committing engine events.
3. **Rules and compendium.** Campaign profiles, core calculations, the full offline SRD 5.2.1 reader, and a source-tagged local importer are implemented. The entire supplied dataset (33,983 entries / 193 books) passed an isolated Rust-store import. Next, add persistent encounter state and reviewed structured mechanics adapters.
4. **Artwork and voice.** Persist generated assets and their specifications with the appropriate game/player state. Evaluate optional Whisper input and model-specific BitNet support separately.

Each milestone should retain campaign reload compatibility and rerun persistence, source-policy, and interface checks.

## Verification boundary

The source-policy scanner detects direct URL literals and known cloud SDK imports in runtime source directories. It is a development guard; dynamically assembled URLs and dependencies require a separate runtime network audit before the full tabletop/AI integration ships.

Verified in this environment: 45 Rust engine tests and 19 UI model/render/display tests; desktop cargo check; TypeScript and the production bundle; catalog consistency; formatting and offline policy. A browser fixture exercised production UI with a local API and all 61 image files, including movement, saves, draft preservation, fog preview, art viewing, local uploads, rules profiles/calculations/search, and compendium imports. The combined Windows development app then passed native verification with an isolated legacy campaign: real IPC, image imports, scene switching, fog/light, dice draft preservation, rules, all six reference collections, and a fresh-profile reopen. The original database hash was unchanged; see [native verification](native-verification.md). Campaign/character save and reopen, dice interaction, and dice history reload were subsequently verified in the user's native app. A native installer launch and Linux packaging remain separate release checks.
