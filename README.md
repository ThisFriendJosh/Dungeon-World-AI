# Dungeon-World-AI

An offline-first desktop virtual tabletop for solo play.

## Current milestone: campaigns, scenes, and saved dice rolls

- Create and pick saved campaigns, with last-save times.
- Add characters and edit their name, class, level, ability scores, HP, armor class, inventory, conditions, and notes.
- Save character changes and reload them across restarts.
- See the latest 50 campaign, character, and scene changes.
- Keep NPCs, places, objects, and notes on a saved tabletop scene board.
- Organize table cards and GM notes with pin, color, and manual turn flags.
- Roll dice with modifiers, advantage/disadvantage, visible faces, and persistent history.
- Get explicit feedback for failed saves and event-log recovery.
- Preserve separate campaigns even when their names normalize to the same slug.

Character statistics are manually recorded. Class features and derived values are entered by the player. New sheets show placeholder values of 10 until the player records the actual values.

The [repository review](docs/repository-review.md) records the roles of Fari, GameMasterAI, Tauri, dnd-data, and optional local-model/voice tools, followed by the next integration milestones. The first Fari scene/index-card adapter is implemented; see [local tabletop scenes](docs/local-tabletop.md). The [local dice tray](docs/dice.md) adds engine-generated rolls with saved history. AI and artwork services currently expose architecture stubs.

## Layout

| Path | Purpose |
| --- | --- |
| apps/desktop | Tauri desktop app and thin command layer |
| apps/vtt-ui | Vite/TypeScript campaign workspace; future Fari adapter |
| services/engine | Independent Rust/SQLite persistence library and regression tests |
| services/ai | Local proposal interfaces and orchestration stubs |
| services/art | Artwork pipeline boundary |
| packages/shared | Reserved for shared schemas |

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
npm run test:ui
npm run check:rust
npm run build:ui
```

The engine tests cover duplicate names, save/reopen round trips, path validation, invalid data, campaign isolation, event-log recovery, and transaction rollback when event insertion fails.

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

```sh
npm run build
```

Tauri builds the UI once using its before-build hook. Windows uses the automatic tauri.windows.conf.json override to select an NSIS .exe installer. Linux uses AppImage and deb targets.

On Windows the root dev/build scripts default to a Rust build cache under LOCALAPPDATA/DungeonWorldAI/cargo-target, avoiding OneDrive read-only directory attributes. An explicit CARGO_TARGET_DIR is respected. Windows installer outputs are under the cache's release/bundle/ folder. Other systems use apps/desktop/src-tauri/target/release/bundle/. Full installer launch verification and Linux packaging are release checks.

In Windows PowerShell, use npm.cmd ci and npm.cmd run dev when the npm.ps1 wrapper is blocked by the local script policy.

## License

AGPL-3.0-or-later. Retain upstream notices when adapting third-party source.
