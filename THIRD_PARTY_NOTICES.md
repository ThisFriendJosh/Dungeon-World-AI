# Third-party notices

## Fari scene adapter

Copyright Fari contributors, including Rene-Pier Deshaies-Gelinas.
License: GNU Affero General Public License, version 3; retain the accompanying LICENSE.

Upstream project: https://github.com/fariapp/fari
Reviewed fork: https://github.com/ThisFriendJosh/fari-app
Pinned source commit: 8aebafb67dbb4ff56e152b5065c2c7b1000baa1f

Adapted files:
- lib/hooks/useScene/IScene.ts
- lib/domains/scene/SceneFactory.tsx
- lib/domains/character/types.tsx
- lib/domains/Id/Id.ts

Local adaptation: apps/vtt-ui/src/fari/scene.ts

Changes: remove React, Immer, browser storage, and application-context dependencies; use browser cryptographic UUIDs; retain the scene/index-card structure and plain-text blocks; restrict this initial adapter to one text block per top-level card; store it through the local Rust campaign engine.

This is a limited scene adapter. General Fari file import, nested cards, advanced block types, collaboration, and the complete Fari UI remain future integrations. The application now has independent local dice and map drawing implementations.

The root license text is AGPLv3, consistent with the application's pre-existing AGPL-3.0-or-later declarations. The previous root file contained GPLv3 text.


## System Reference Document 5.2.1

This work includes material from the System Reference Document 5.2.1 (“SRD 5.2.1”) by Wizards of the Coast LLC, available at https://www.dndbeyond.com/srd. The SRD 5.2.1 is licensed under the Creative Commons Attribution 4.0 International License, available at https://creativecommons.org/licenses/by/4.0/legalcode.
Original PDF source: https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf

Bundled files: apps/vtt-ui/public/rules/srd-5.2.1.pdf, srd-5.2.1.json, attribution.txt, and attribution.html. The source PDF is unchanged; searchable text was extracted for the offline reader. The SRD content retains its CC BY 4.0 license.

## Supplied artwork

62 source images were provided for this project. Runtime WebP copies retain source dimensions. Portrait view boxes display regions of the original illustrations. The assignment list and source hashes are in packages/shared/assets/catalog.json and docs/art-placement.md. Printed homebrew statistics are reference text, separate from the official SRD and executable mechanics.

## Local compendium imports

The application includes an adapter for the dnd-data JSON format; no dnd-data dataset or package source is bundled. Imported entries retain their publisher and source-book metadata in local campaign storage.

## Optional local image model

The application can load Stability AI's SD-Turbo through an existing Python/diffusers environment. Model weights are downloaded separately by explicit setup and are not bundled in this repository or native build. The pinned model's README and LICENSE.md are retained in its model folder. Model terms: https://huggingface.co/stabilityai/sd-turbo/blob/main/LICENSE.md. Model card: https://huggingface.co/stabilityai/sd-turbo. These terms are separate from the application license and supplied campaign artwork.
