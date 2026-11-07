# Oak tabletop canvas

The Tabletop view provides dark oak rails, brass trim, a recessed map, an inspector, scene selection, and an expandable felt dice tray. The supplied candlelit table image provides the welcome/tabletop art direction. Sheets and codex entries use parchment surfaces.

Focus table hides the campaign sidebar and heading to expand play space. Exit focus restores them. Existing cards and GM notes remain in their drawer.

## Scenes and artwork

Each campaign supports 40 saved scenes. New scene starts a location, Save scene stores it, and Saved scenes opens a stored location. Switching scenes prompts before discarding edits. The active scene returns after reopening.

62 supplied source images become 61 unique WebP files and 72 catalog entries including portrait views: 11 battle maps, 11 scenes, 16 portraits, four atlas maps, 29 reference folios, and one table-style image. Originals are unchanged. Runtime copies retain dimensions; portrait views select image regions without altering sources.

Art & codex provides category filters, search, full-source viewing, and background/token actions. Sewer, desert, pond, and forest maps are battle backgrounds. Narrative illustrations can be scene backdrops; their positions are described as markers. World maps retain their own scale. Secret-door floor plans stay in GM references and are not offered as player backgrounds. See [all assignments](art-placement.md).

Import background / Import sprite accept local raster images. SVG is excluded. The first frame is converted to PNG, scaled to at most 2048 pixels per edge, and reduced to fit 4 MB. Inputs may be up to 20 MB and 16384 pixels per edge. Nothing is uploaded. Each campaign supports 100 imported images / 50 MB. A replacement background preserves pieces, drawings, and fog and resets the image transform.

## Scale and pieces

Each application square is 5 × 5 feet. Set rows/columns (8–100), toggle grid and snap independently, and adjust opacity. Image fit, scale (25–400%), and offsets align a background.

Forest illustrations contain printed grids. Starting canvas sizes are supplied, but spacing varies within the illustrations. Verify alignment before measuring movement; an exact application grid cannot remove irregular printed lines.

Drag pieces or nudge with arrow keys: one square with snap, one foot without. Complete footprints stay inside the board. Pan, zoom, and Fit map navigate larger boards. Distance uses equal-cost diagonals; terrain and movement budgets remain table decisions.

Player, NPC, Monster, Object, and Treasure pieces have stable IDs, position, footprint, color, sprite, notes, state labels, optional HP, rotation, GM visibility, locks, and light radii. Up to 100 pieces fit in a scene. Duplicate creates a new ID.

Place a character copies its name and HP as a starting snapshot. Token HP remains independent of the sheet. A portrait supplies artwork and a suggested piece type without importing printed statistics or homebrew effects.

## Drawing, fog, and lighting

Draw creates freehand strokes; Rectangle and Ellipse create outlines. Undo/Redo retains 30 checkpoints. Scenes support 200 drawings / 2000 points per drawing.

Cover all enables fog. Reveal all disables it while retaining marks. Reveal fog and Conceal fog paint circular regions; later marks override earlier ones. Up to 500 marks are saved. GM view is translucent; Player preview uses opaque fog and excludes hidden pieces.

Daylight, Dim light, and Darkness set ambient light. Bright and total dim-edge radii use feet and move with emitting pieces. Light passes through walls. Wall occlusion, creature vision, and automatic exploration are not implemented.

Player preview is a local GM map preview, not a separate player session or an access-control boundary. The inspector and GM notes remain on the same device.

## Persistence

Save scene stores art IDs/transforms, positions, drawing, fog, light, notes, and states together. Optimistic revisions reject stale saves/selections. A scene change and its event commit atomically.

Imported PNG bytes live separately in SQLite; events contain metadata. Migration 003 adds old active scenes to the scene library without changing cards. Card-only scenes gain a five-foot board. Dice rolls preserve unsaved map drafts.

The catalog supplies stable IDs to frontend and engine. Run node scripts/sync-art-catalog.mjs after editing it. Tests compare the generated TypeScript data with the catalog.

## Verification

45 Rust and 19 UI tests cover persistence, editions, compendium imports, image IDs, legacy upgrades, bounds, fog ordering, escaping, and separate dice/scene forms. TypeScript, desktop compilation, offline policy, catalog generation, and the production UI bundle are checked.

A browser fixture runs production UI with an isolated API: all 61 images decode; movement, dice draft preservation, saves, fog preview, undo, art reading, portrait placement, local image upload, edition saves/calculations, offline SRD search, and compendium filtering/details/imports pass. Desktop and mobile layouts were inspected. Requests stay on loopback; the fixture is excluded from the app.

The complete supplied dnd-data archive was separately imported using the actual Rust store: 33,983 entries / 193 source-book labels.

The Windows native development app passed the save/reopen checks using an isolated copy of an existing campaign. Character values and both GM cards were preserved; two scenes, imported background/sprite images, pieces, fog/light, dice, rules, and all 33,983 compendium entries survived a fresh process and fresh webview profile. The original database hash was unchanged. See [native verification](native-verification.md). Installer launch and Linux packaging remain release checks.
