# Safe import playbook: Fari + GameMasterAI

This repo is wired so:
- `apps/vtt-ui` is the UI app (future Fari fork destination).
- `services/ai` is the offline AI service boundary.
- Tauri depends on `apps/vtt-ui` dev/build scripts and output paths.

## 1) Import Fari into `apps/vtt-ui`

### Hard constraints (do not break)
- Keep `apps/vtt-ui/package.json` package name as `@dwa/vtt-ui`.
- Keep scripts:
  - `dev` should run a server on port `5173`.
  - `build` should output to `dist`.
- If these change, update `apps/desktop/src-tauri/tauri.conf.json` accordingly.

### Recommended import method
Use a temporary clone and rsync copy (or git subtree if preferred).

1. Backup current UI folder:
   - `mv apps/vtt-ui apps/vtt-ui.pre-fari`
2. Clone Fari to temp location.
3. Copy source into `apps/vtt-ui`.
4. Re-apply Tauri-compatible package scripts/name.
5. Remove cloud-only runtime integrations.
6. Verify no external network calls at runtime.

### After import checklist
- `npm run check` passes.
- Tauri still points to:
  - `beforeDevCommand: npm run dev --workspace @dwa/vtt-ui`
  - `devUrl: http://localhost:5173`
  - `frontendDist: ../../vtt-ui/dist`

## 2) Import GameMasterAI into `services/ai`

### Hard constraints
- Treat imported code as reference/vendor.
- Final runtime must be offline only (no cloud API calls).
- AI outputs are proposals; engine remains canonical source of truth.

### Recommended layout
- `services/ai/vendor/gamemasterai/` for imported snapshot.
- `services/ai/src/` for adapted local modules.

### Adaptation rules
- Remove external API SDK usage from runtime path.
- Replace remote HTTP calls with local adapters/stubs.
- Keep attribution in `services/ai/ATTRIBUTION.md`.

## 3) Offline enforcement
Run:

```bash
npm run policy:offline
```

This scans for known cloud SDK imports and direct external URLs in runtime code paths.
