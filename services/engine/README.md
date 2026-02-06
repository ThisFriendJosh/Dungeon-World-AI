# services/engine

Engine persistence assets for v0.

- `migrations/001_init.sql` defines campaign, character, and event tables.
- Runtime currently lives in Tauri Rust commands in `apps/desktop/src-tauri/src/main.rs`.
- Future milestone: split to standalone engine service process with explicit API.
