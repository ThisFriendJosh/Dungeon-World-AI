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

This is a limited scene adapter. General Fari file import, nested cards, advanced block types, drawing, dice, collaboration, and the complete Fari user interface are future integrations.

The root license text is AGPLv3, consistent with the application's pre-existing AGPL-3.0-or-later declarations. The previous root file contained GPLv3 text.
