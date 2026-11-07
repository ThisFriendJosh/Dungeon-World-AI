# Local tabletop scenes

The first Fari integration adapts its scene and index-card model into the existing offline campaign workspace. Each campaign has one current scene board with a name, notes, and up to 50 cards across two groups: Table cards and GM notes.

Cards support a type label, title, color, text notes, a pin, and an "Acted this turn" flag. Pinning places a card first in the displayed group. Turn flags are manual bookkeeping; initiative and action economy remain player-managed.

Select Tabletop in a loaded campaign, enter the scene name, add cards, and choose Save scene. Edits remain local to the form until saved. Switching campaign or workspace view prompts before discarding unsaved edits. A failed save keeps the scene edits visible.

The GM notes group organizes the solo player's own notes. It is not a multiplayer authorization system. Both groups are saved with the campaign.

## Compatibility and persistence

The scene structure is adapted from Fari's IScene and IIndexCard definitions at the source commit recorded in THIRD_PARTY_NOTICES.md. This initial adapter supports version-3 structures, top-level cards, and one plain-text notes block per card. General Fari file import/export is not exposed.

Migration 002 adds a tabletops table to each existing campaign database. Original campaigns, characters, and events retain their existing schema and values. Each saved scene and its scene.saved event commit in one transaction, followed by the existing recoverable event-log mirror update.

Scene revisions reject stale saves from another window. The failed save keeps the user's edits on screen; Reload/Refresh remains an explicit choice because it discards the stale draft.

## Verification

Run from the repository root:

```sh
npm run check
npm run test:ui
npm run test:engine
npm run check:rust
npm run build:ui
```

The engine tests exercise scene reloads, public/GM card preservation, legacy save migration, invalid content, campaign isolation, stale revisions, and atomic rollback when event insertion fails. Adapter tests cover independent scenes, draft isolation, and escaping untrusted text.

For the manual desktop check: create an NPC card and a location card, save, close/reopen the application, load the same campaign, and confirm both cards and their notes return. Check that a canceled unsaved-change prompt preserves edits. Interactive scene testing and installer packaging remain release checks.

## Windows development

The root dev/build scripts now select a Rust target directory under LOCALAPPDATA/DungeonWorldAI/cargo-target on Windows when CARGO_TARGET_DIR is unset. This avoids the read-only directory attributes encountered in a OneDrive checkout. An explicit CARGO_TARGET_DIR is respected.

The built executable is under that target directory's debug folder for dev builds; installer outputs are under its release/bundle folder. Other operating systems retain Cargo's normal target location.

In Windows PowerShell use npm.cmd if the npm.ps1 wrapper is blocked by the local script policy.
