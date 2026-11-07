import { invoke } from "@tauri-apps/api/core";
import { copyScene, makeScene, makeSceneCard } from "./fari/scene";
import type { FariScene, TabletopState } from "./fari/scene";
import type { Saved } from "./types";
import { escapeHtml as e } from "./html";

type Context = {
  campaignId: string;
  action(work: () => Promise<void>): Promise<void>;
  onSaved(warning: string | null): Promise<void>;
  onDirty(): void;
  onError(message: string): void;
  render(): void;
  isBusy(): boolean;
};

export class SceneEditor {
  private saved: TabletopState = { scene: null, revision: 0, updatedAt: null };
  private draft: FariScene = makeScene();
  private audience: "public" | "private" = "public";
  dirty = false;

  load(value: TabletopState) {
    this.saved = structuredClone(value);
    this.draft = value.scene ? copyScene(value.scene) : makeScene();
    this.dirty = false;
  }
  discard() { this.load(this.saved); }
  html(): string {
    const cards = this.draft.indexCards[this.audience];
    const count = this.draft.indexCards.public.length + this.draft.indexCards.private.length;
    return `
      <section class="tabletop-panel">
        <form id="sceneForm">
          <div class="section-heading"><div><p class="eyebrow">Tabletop scene</p><h2>Scene board</h2></div>
            <button class="primary" type="submit">Save scene</button></div>
          <p class="subtle">Keep NPCs, places, objects, and clues on index cards. Save the scene to keep it with this campaign.</p>
          <fieldset><legend class="sr-only">Scene details</legend>
            <label>Scene name<input id="sceneName" required maxlength="120" value="${e(this.draft.name)}"></label>
            <label class="scene-notes">Scene notes<textarea id="sceneNotes" rows="3" maxlength="10000">${e(this.draft.notes)}</textarea></label>
            <div class="scene-toolbar">
              <div class="scene-groups" aria-label="Card groups">
                <button type="button" data-scene-group="public" aria-pressed="${this.audience === "public"}">Table cards (${this.draft.indexCards.public.length})</button>
                <button type="button" data-scene-group="private" aria-pressed="${this.audience === "private"}">GM notes (${this.draft.indexCards.private.length})</button>
              </div>
              <button type="button" id="addSceneCard" ${count >= 50 ? "disabled" : ""}>Add card</button>
            </div>
            <div class="scene-cards">${cards.length ? cards.slice().sort((a,b) => Number(b.pinned)-Number(a.pinned)).map(card => `
              <article class="scene-card" data-card-id="${e(card.id)}" style="--card-accent:${/^#[0-9a-f]{6}$/i.test(card.color) ? card.color : "#d6b973"}">
                <div class="card-heading"><label>Card type<input data-card-field="titleLabel" maxlength="40" required value="${e(card.titleLabel)}" placeholder="NPC, Location, Object, Note"></label>
                  <label class="card-color">Color<input data-card-field="color" type="color" value="${e(card.color)}"></label></div>
                <label>Title<input data-card-field="title" required maxlength="120" value="${e(card.title)}"></label>
                <label>Notes<textarea data-card-field="notes" rows="5" maxlength="10000">${e(card.blocks[0].value)}</textarea></label>
                <div class="card-options">
                  <label><input data-card-field="pinned" type="checkbox" ${card.pinned ? "checked" : ""}>Pin first</label>
                  <label><input data-card-field="playedDuringTurn" type="checkbox" ${card.playedDuringTurn ? "checked" : ""}>Acted this turn</label>
                  <button type="button" data-remove-card="${e(card.id)}" class="quiet">Remove card</button>
                </div>
              </article>`).join("") : '<p class="empty-note">Add a card for an NPC, location, object, or note in this scene.</p>'}</div>
          </fieldset>
          <p class="subtle scene-save-note">${this.saved.updatedAt ? "Last saved " + e(new Date(this.saved.updatedAt).toLocaleString()) : "This scene has not been saved yet."}</p>
        </form>
      </section>`;
  }
  bind(root: HTMLElement, context: Context) {
    const form = root.querySelector<HTMLFormElement>("#sceneForm");
    if (!form) return;
    const read = () => {
      this.draft.name = root.querySelector<HTMLInputElement>("#sceneName")!.value;
      this.draft.notes = root.querySelector<HTMLTextAreaElement>("#sceneNotes")!.value;
      for (const card of this.draft.indexCards[this.audience]) {
        const article = root.querySelector<HTMLElement>('[data-card-id="' + card.id + '"]');
        if (!article) continue;
        const input = (key: string) => article.querySelector<HTMLInputElement>('[data-card-field="' + key + '"]')!;
        card.titleLabel = input("titleLabel").value;
        card.title = input("title").value;
        card.color = input("color").value;
        card.pinned = input("pinned").checked;
        card.playedDuringTurn = input("playedDuringTurn").checked;
        card.blocks[0].value = article.querySelector<HTMLTextAreaElement>('[data-card-field="notes"]')!.value;
      }
    };
    const changed = () => { read(); this.dirty = true; context.onDirty(); };
    form.addEventListener("input", changed);
    form.addEventListener("change", event => {
      if ((event.target as HTMLInputElement).dataset.cardField === "pinned") { read(); context.render(); }
    });
    form.addEventListener("submit", event => {
      event.preventDefault();
      read();
      const scene = copyScene(this.draft);
      scene.name = scene.name.trim();
      scene.lastUpdated = Math.floor(Date.now()/1000);
      void context.action(async () => {
        const result = await invoke<Saved<TabletopState>>("save_tabletop", {
          campaignId: context.campaignId, scene, expectedRevision: this.saved.revision
        });
        this.load(result.value);
        await context.onSaved(result.warning);
      });
    });
    root.querySelectorAll<HTMLButtonElement>("[data-scene-group]").forEach(button => button.addEventListener("click", () => {
      if (context.isBusy()) return;
      read();
      this.audience = button.dataset.sceneGroup as "public" | "private";
      context.render();
    }));
    root.querySelector("#addSceneCard")?.addEventListener("click", () => {
      if (context.isBusy()) return;
      read();
      if (this.draft.indexCards.public.length + this.draft.indexCards.private.length >= 50) {
        context.onError("A scene can contain up to 50 cards."); return;
      }
      const card = makeSceneCard(); card.title = "New note";
      this.draft.indexCards[this.audience].push(card);
      this.dirty = true; context.onDirty(); context.render();
    });
    root.querySelectorAll<HTMLButtonElement>("[data-remove-card]").forEach(button => button.addEventListener("click", () => {
      if (context.isBusy()) return;
      read();
      this.draft.indexCards[this.audience] = this.draft.indexCards[this.audience].filter(card => card.id !== button.dataset.removeCard);
      this.dirty = true; context.onDirty(); context.render();
    }));
  }
}
