# Dungeon-World-AI

Offline-first desktop virtual tabletop for solo play.

## v0 Scope (implemented)

- Desktop shell via **Tauri** (`apps/desktop`).
- Web UI scaffold via **Vite** (`apps/vtt-ui`) as a future Fari-based fork destination.
- Campaign persistence engine in Rust (inside Tauri commands), with:
  - campaign directory per campaign slug
  - `campaign.sqlite`
  - append-only `events.log`
- Character persistence and reload across restarts.
- Stub service boundaries for `services/ai` and `services/art`.

## Monorepo layout

```text
apps/
  desktop/      # Tauri wrapper and Rust command layer
  vtt-ui/       # Web UI scaffold (future Fari fork base)
services/
  engine/       # SQL migrations and engine-related assets
  ai/           # Offline AI architecture stub + vendor imports
  art/          # Offline art pipeline stub
packages/
  shared/       # Shared schemas/types stub
docs/
  import-playbook.md
scripts/
  import_fari.sh
  import_gamemasterai.sh
  offline_policy_check.sh
```

## Safe import workflow (Fari + GameMasterAI)

1. Read `docs/import-playbook.md`.
2. Import Fari into `apps/vtt-ui`:

```bash
npm run import:fari -- https://github.com/ThisFriendJosh/fari-app.git
```

3. Import GameMasterAI snapshot into `services/ai/vendor/gamemasterai`:

```bash
npm run import:gamemasterai -- https://github.com/ThisFriendJosh/gamemasterai.git
```

4. Enforce offline guardrails:

```bash
npm run policy:offline
```

## Data model (v0)

Each campaign is stored in app data under:

```text
campaigns/<campaign-slug>/
  campaign.sqlite
  events.log
```

SQLite tables:
- `campaigns`
- `characters`
- `events`

Event sourcing path:
- canonical writes go to SQLite `events`
- same event appended as JSONL to `events.log`

## Local development

### Prerequisites

- Node.js 20+
- Rust stable toolchain
- Linux desktop deps for Tauri (or Windows build chain)

### Install

```bash
npm install
```

### Run desktop app in dev mode

```bash
npm run dev
```

### Optional: override app data directory (useful for testing)

```bash
DWA_DATA_DIR=/absolute/path/to/local-data npm run dev
```

## Build packages

### Windows (.exe installer via NSIS)

```bash
npm run build
```

Output artifact appears under `apps/desktop/src-tauri/target/release/bundle/nsis/`.

### Linux (AppImage + deb)

```bash
npm run build
```

Output artifacts appear under `apps/desktop/src-tauri/target/release/bundle/`.

> Flatpak packaging is planned; AppImage is configured now for the immediate milestone.

## License

AGPL-3.0-or-later (required when incorporating AGPL Fari-derived UI code).
