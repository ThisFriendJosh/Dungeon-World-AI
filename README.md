# Dungeon-World-AI

An offline-first desktop virtual tabletop for solo play.

## Windows beta installer

Open **Dungeon-World-AI-0.2.0-beta.1-Windows-Setup.msi**, choose **Install**, then **Finish** to open guided local AI setup. Desktop and Start menu shortcuts open the app. No developer tools or API keys are needed. The app and Scrying Pane are bundled; the Dungeon Master model and optional AI art download once during setup. Campaigns and models stay on disk when the app is uninstalled.

See the [short beta guide](docs/beta-start-here.txt) and [installer build and verification notes](docs/beta-installer.md). This is an unsigned beta; the normal Windows install and initial downloads need a clean-PC test before wider distribution.

## Current milestone: local AI Dungeon Master integration

- Open the Dungeon Master tab, choose a character, and send a player action to a local GGUF model.
- Keep accepted actions and narration in the campaign database across restarts.
- Supply the narrator with recorded character values, public scene cards, the selected rules profile, and actual saved dice rolls.
- Keep mechanical changes manual; reject malformed or stale replies before saving them.
- Search local campaign/source packs and compendium descriptions; give the DM read-only table/character inspection and engine arithmetic.
- Create and save procedural PNG/SVG tokens, and clean up individual history entries with confirmation and Restore.

See [AI Dungeon Master setup and verification](docs/ai-dungeon-master.md) for development launchers, current limits, and real-model checks. The beta installer bundles its own runtimes and guides the separate model downloads.

The [DM workspace guide](docs/dm-workspace.md) covers source imports, grid awareness, calculations, visual assets and history edits. The [beta feedback roadmap](docs/beta-feedback-roadmap.md) tracks survey priorities including combat, multiplayer and save recovery.

- Create and pick saved campaigns, with last-save times.
- Add characters and edit their name, class, level, ability scores, HP, armor class, inventory, conditions, and notes.
- Save character changes and reload them across restarts.
- See the latest 50 campaign, character, and scene changes.
- Keep NPCs, places, objects, and notes on a saved tabletop scene board.
- Organize table cards and GM notes with pin, color, and manual turn flags.
- Keep several saved scenes with a 5-foot grid, imported backgrounds and sprites, movable pieces, notes, HP, and state labels.
- Draw on the map, paint fog, set circular light radii, and roll from the table's dice tray.
- Browse 62 supplied source images in Art & codex; use 11 battle maps, 11 scene backdrops, and 16 portrait views.
- Read and search the complete offline SRD 5.2.1, select a campaign rules profile, and calculate core modifiers, concentration, and revised exhaustion.
- Import local dnd-data JSON collections with publisher/book/year filters and separate same-name source entries.
- Roll dice with modifiers, advantage/disadvantage, visible faces, and persistent history.
- Get explicit feedback for failed saves and event-log recovery.
- Preserve separate campaigns even when their names normalize to the same slug.

Character statistics are manually recorded. Core calculations are available in Rules; class features, spells, encounter turns, conditions, and derived sheet values remain table decisions. New sheets show placeholder values of 10 until the player records the actual values.

The [repository review](docs/repository-review.md) records the roles of Fari, GameMasterAI, Tauri, dnd-data, and optional local-model/voice tools, followed by the next integration milestones. The first Fari scene/index-card adapter is implemented; see [local tabletop scenes](docs/local-tabletop.md). The [local dice tray](docs/dice.md) adds engine-generated rolls with saved history. The [oak tabletop canvas](docs/tabletop-canvas.md) adds maps, scene selection, image import, tokens, drawing, fog, and light. The supplied artwork is packaged and assigned in [art placement](docs/art-placement.md). [Rules profiles and the local compendium](docs/rules-engine.md) describe edition boundaries and current automation coverage. The native AI narrator now has an initial local runtime and persistence implementation; broader AI modules and the artwork service remain architecture stubs.

## Layout

| Path | Purpose |
| --- | --- |
| apps/desktop | Tauri desktop app and thin command layer |
| apps/vtt-ui | Vite/TypeScript campaign workspace; future Fari adapter |
| services/engine | Independent Rust/SQLite persistence library and regression tests |
| services/ai | Rust local-model runtime and narrator prompt; future TypeScript module interfaces |
| services/art | Artwork pipeline boundary |
| packages/shared | Shared schemas and the bundled artwork catalog |

## Local development

Prerequisites: Node.js 20+, npm, a stable Rust toolchain, and the [Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/).

From the repository root:

```sh
npm ci
npm run dev
```

The desktop app uses the UI development server on port 5173. The port is strict to keep the desktop and UI connected.

Set an alternate app-data root for development:

```powershell
$env:DWA_DATA_DIR = "C:\DWA-Test-Data"
npm run dev
```

```sh
DWA_DATA_DIR=/absolute/path/to/local-data npm run dev
```

A standalone browser preview is available with:

```sh
npm run dev --workspace @dwa/vtt-ui
```

Campaign commands require the desktop app. The browser preview explains this in its status bar.

## Checks

```sh
npm run check
npm run test:engine
npm run test:ai
npm run test:ui
npm run check:rust
npm run build:ui
```

The engine tests cover duplicate names, save/reopen round trips, path validation, invalid data, campaign isolation, rules profiles, reference imports, event-log recovery, and transaction rollback. The combined Windows development app also passed [native save/reopen verification](docs/native-verification.md) using an isolated copy of an existing campaign.

The offline source check runs on Windows and Linux using Node. It scans runtime source for known cloud SDK imports and direct URL literals; dependency and runtime network audits remain part of future integration work.

## Persistence

The OS app-data folder, or DWA_DATA_DIR, contains:

```text
campaigns/<campaign-slug>/
  campaign.sqlite
  events.log
```

Existing v0 databases use the same tables and JSON columns. A newly created duplicate name receives a distinct slug rather than replacing an existing campaign.

SQLite is the canonical source of truth. Campaign/character mutations and their events commit in one transaction. The JSONL log is a derived mirror, ordered by event sequence and rebuilt after a mutation or when a campaign is loaded. If refreshing that mirror fails, the app reports that the database save succeeded and retries the mirror on the next load.

## Desktop packages

For an embedded Windows development build, run `npm.cmd run build:native`, then open `Start-Dungeon-World-AI.cmd` from File Explorer. See [native imports and clipboard](docs/native-imports-and-clipboard.md) for local AI connection diagnostics, Compendium document formats, and test-save configuration.

```sh
npm run build
```

Tauri builds the UI once using its before-build hook. Windows uses the automatic tauri.windows.conf.json override to select an NSIS .exe installer. Linux uses AppImage and deb targets.

On Windows the root dev/build scripts default to a Rust build cache under LOCALAPPDATA/DungeonWorldAI/cargo-target, avoiding OneDrive read-only directory attributes. An explicit CARGO_TARGET_DIR is respected. Windows installer outputs are under the cache's release/bundle/ folder. Other systems use apps/desktop/src-tauri/target/release/bundle/. Full installer launch verification and Linux packaging are release checks.

In Windows PowerShell, use npm.cmd ci and npm.cmd run dev when the npm.ps1 wrapper is blocked by the local script policy.

## Timers, browser and 3D dice

The first campaign tabs are Game Timers & Calculators, Scrying Pane, and 3D Dice. Timers save real/game clocks, calculators preview game arithmetic, and animated dice retain the engine's recorded results. The CEF browser installs separately through `Setup-Scrying-Pane.cmd`; website browsing is explicit and separate from offline gameplay. See [game tools, Scrying Pane and dice](docs/game-tools-scrying-dice.md).

## Artwork

Art & codex supports campaign image uploads, multiple categories, and editable custom tags. The token workshop draws sprites with a local image model and keeps frame/color controls separate. Open `Setup-Local-Art.cmd` once to download the model; imports work without it. See [art library and AI sprites](docs/art-library-and-sprites.md).

## License

AGPL-3.0-or-later. Retain upstream notices when adapting third-party source.
