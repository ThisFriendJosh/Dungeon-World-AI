# Rules profiles, offline SRD, and local compendium

Verified against official sources on 2026-11-07.

## Baseline and coverage

The revised core comprises the 2024 Player's Handbook and Dungeon Master's Guide and the 2025 Monster Manual. D&D Beyond now labels the revised rules 5.5e and retains 5e for 2014 rules. The latest open System Reference Document is 5.2.1, released May 1, 2025. The SRD does not contain every expansion or commercial book.

Official sources:

- https://www.dndbeyond.com/srd
- https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf

The Rules view bundles all 364 pages as an unchanged PDF and extracted searchable text, with chapter/page navigation and attribution. Everything is available offline. The PDF preserves the published layout; extracted text retains page boundaries.

Original PDF: 6,031,375 bytes. SHA-256: 8974902d109d6e63672d7c490bde9ccf052410503d9cfa768237154fbc5e3d87.

## Profiles and compatibility

| Stored profile | Behavior |
| --- | --- |
| 5e-manual | Existing campaigns remain unassigned until the table chooses an edition. Edition-specific calculations remain manual. |
| 5e-2014 | Legacy concentration calculation; revised exhaustion is not applied. |
| 5e-2024 | Default for new campaigns; revised concentration and exhaustion calculations. |

The revised UI label is “5.5e · revised 2024 rules.” Storage identifiers remain independent of branding.

Migration 004 backfills existing campaigns with 5e-manual. Creating a new campaign explicitly initializes 5e-2024. Choosing a profile does not convert subclasses, levels, HP, abilities, equipment, or spells. An illustrated character sheet never replaces an existing character's saved values.

House rules are separate: flanking advantage off, potion as a Bonus Action on, maximize one critical die off. Preferences are saved but are not yet applied by automated combat resolution.

Profile saves use optimistic revisions. Settings and their rules.changed event commit together, with the same event-log mirror recovery behavior as other campaign changes.

## Implemented mechanics

| Capability | Behavior |
| --- | --- |
| Ability modifier | Floor((score − 10) / 2), including negative odd scores |
| Proficiency | Levels 1–20, +2 through +6 |
| Proficient check / attack | Ability modifier + proficiency; excludes expertise, items, situational bonuses, and exhaustion |
| Spell save DC | 8 + ability modifier + proficiency |
| Damage concentration check | Minimum DC 10 or half damage rounded down; revised profile caps DC at 30; zero damage requires no damage check |
| Revised exhaustion | −2 per level to D20 Tests; speed reduced 5 ft per level; six levels means death |
| Legacy exhaustion | Manual; revised numeric effects are never substituted |
| Dice | Engine-generated faces, modifiers, advantage/disadvantage, and stored results |
| Positioning | Five-foot squares, equal-cost diagonals, footprint bounds; terrain and movement budgets remain manual |

The calculator previews a ruling. It does not roll a concentration save or apply HP, exhaustion, speed, death, conditions, or spell effects to a character.

Class features, spellcasting, monster abilities, initiative, actions, rests, concentration state, conditions, and equipment effects remain manually adjudicated. A complete reference reader is not a complete automated rules engine.

## dnd-data

Reviewed repository: https://github.com/ThisFriendJosh/dnd-data

The supplied archive was parsed and imported through the actual Rust store into an isolated audit campaign.

| Dataset | Entries |
| --- | ---: |
| Backgrounds | 405 |
| Classes | 134 |
| Items | 15,749 |
| Monsters | 11,463 |
| Species | 383 |
| Spells | 5,849 |
| Total | 33,983 |

There are 193 distinct source-book labels. Archive SHA-256: d22fc3713e0618a7a973dc8d33814f23d65291e4017920dc9df66281333fe761.

In Compendium, select the six JSON files from the extracted data folder. No npm package or repository code runs. Each import adds a collection; it does not replace an earlier collection.

Entries retain name, description, publisher, book, and properties. Missing/null optional text becomes empty text. Properties remain JSON and are displayed as escaped text. Same-name entries from different sources remain separate.

Search by name/publisher, dataset, source book, and explicit source-year tag. Titles containing 2014 get a legacy tag; 2024/2025 get a revised tag; others remain unassigned. These describe a title, not verified edition compatibility. Unmarked PHB/MM titles are not guessed to be the latest edition.

Imports stay in the campaign database. Limits: 20,000 entries / 40 MB per file; 16 collections / 50,000 entries / 100 MB per campaign. Each file and its event commit atomically. Earlier successful files remain saved if a later file fails, and the UI reports this. Events contain collection metadata rather than descriptions.

The repository's MIT label does not establish redistribution terms for every commercial and third-party source represented in its data. The public build bundles the CC BY 4.0 SRD and an importer for the wider local collection. The wider dataset is excluded from source control and the public build.

A command-line importer is available:

```sh
cargo run --manifest-path services/engine/Cargo.toml --example import_compendium -- /local/campaigns existing-campaign-slug /extracted/dnd-data/data
```

Use --new-audit instead of a campaign slug to create a separate verification campaign. The command never selects the live app data directory automatically.

## Next mechanics milestone

1. Persist encounters: initiative, rounds, turns, movement, Action, Bonus Action, and Reaction.
2. Resolve explicit attack/save/damage proposals with edition, advantage sources, cover, criticals, HP, concentration, and conditions recorded as events.
3. Add reviewed class, species, item, feat, and spell adapters against pinned sources, preserving manual overrides and edition identity.
4. Connect spell slots, rests, concentration, and effect durations to encounter state.
5. Add wall occlusion and creature vision after positioning and encounter effects have a canonical model.

Illustrated and free-form descriptions are reference text. Executable effects require reviewed structured rules and explicit state transitions.
