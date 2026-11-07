// SPDX-License-Identifier: AGPL-3.0-or-later
// Adapted from Fari's IScene/IIndexCard and SceneFactory. See THIRD_PARTY_NOTICES.md.
// This first adapter supports top-level cards with plain-text notes.
export type SceneTextBlock = {
  id: string; type: "Text"; label: string; value: string; meta: Record<string, never>;
};
export type SceneCard = {
  id: string; titleLabel: string; title: string; color: string;
  playedDuringTurn: boolean; pinned: boolean;
  blocks: SceneTextBlock[]; subCards: SceneCard[]; sub: boolean;
};
export type FariScene = {
  id: string; name: string; group?: string;
  indexCards: { public: SceneCard[]; private: SceneCard[] };
  version: 3; lastUpdated: number; notes: string;
};
export type TabletopState = { scene: FariScene | null; revision: number; updatedAt: string | null };
export const makeScene = (): FariScene => ({
  id: crypto.randomUUID(), name: "Current scene",
  indexCards: { public: [], private: [] },
  version: 3, lastUpdated: Math.floor(Date.now() / 1000), notes: ""
});
export const makeSceneCard = (): SceneCard => ({
  id: crypto.randomUUID(), titleLabel: "Note", title: "", color: "#d6b973",
  playedDuringTurn: false, pinned: false, sub: false, subCards: [],
  blocks: [{ id: crypto.randomUUID(), type: "Text", label: "Notes", value: "", meta: {} }]
});
export const copyScene = (scene: FariScene): FariScene => structuredClone(scene);
