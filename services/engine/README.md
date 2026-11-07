# Campaign persistence engine

Standalone Rust library used by the Tauri command layer.

- src/lib.rs defines campaign, character, and event operations.
- migrations/001_init.sql remains compatible with existing v0 saves.
- src/tests.rs exercises persistence, validation, transaction rollback, isolation, and log recovery using disposable fixtures under target/.

Run cargo test --manifest-path services/engine/Cargo.toml from the repository root.

SQLite is canonical; events.log is a rebuildable JSONL mirror. Failed mirror refreshes return warnings alongside successfully committed mutations. Records and their events share a transaction.

The UI uses manual 5e character sheet values. This library validates basic data shapes and numeric ranges; a future rules engine will adjudicate derived game mechanics.
