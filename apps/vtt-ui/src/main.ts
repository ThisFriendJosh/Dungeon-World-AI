import { invoke, isTauri } from "@tauri-apps/api/core";
import type { CampaignSummary, CharacterSummary, CampaignEvent, Saved } from "./types";
import "./style.css";
import "./tabletop.css";
import {ArtLibrary} from "./art-library";
import type {ArtEntry} from "./art-library";
import {TABLE_ART} from "./bundled-art";
import "./art-library.css";
import {RulesPanel} from "./rules-panel";
import type {RulesState} from "./rules-panel";
import "./rules.css";
import {Compendium} from "./compendium";
import "./compendium.css";
import { SceneEditor } from "./tabletop";
import type { TabletopState } from "./fari/scene";
import { escapeHtml as escape } from "./html";
import type {SceneSummary,SceneImage} from "./board-model";
import { DicePanel } from "./dice-panel";
import type { DiceRoll } from "./dice-types";
import { describeRoll } from "./dice-format";
import { DmPanel } from "./dm-panel";
import type { DmHistory, DmRuntimeStatus } from "./dm-types";
import "./dm.css";
import {KnowledgePanel} from "./knowledge-panel";
import type {KnowledgeSource} from "./knowledge-panel";
import {HistoryPanel} from "./history-panel";
import {AssetWorkshop} from "./asset-workshop";
import type {GeneratedAsset,ArtRuntimeStatus} from "./asset-workshop";
import "./workspace-tools.css";
import {GameToolsPanel} from "./game-tools";
import type {GameTools} from "./game-tools";
import "./game-tools.css";
import {WORKSPACE_TABS} from "./workspace-tabs";
import type {WorkspaceView} from "./workspace-tabs";
import {ScryingPane} from "./scrying-pane";
import type {ScryingBookmark} from "./scrying-pane";
import "./scrying-pane.css";
import "./dice-animation.css";
import {installClipboard} from "./clipboard";
import {compendiumWorkspace} from "./compendium-workspace";
import type {CompendiumSection} from "./compendium-workspace";
const knowledge=new KnowledgePanel();
const historyPanel=new HistoryPanel();
const assetWorkshop=new AssetWorkshop();
const dm = new DmPanel();
const dice = new DicePanel();
const tabletop = new SceneEditor();
const rules = new RulesPanel();
const compendium = new Compendium();
const artLibrary = new ArtLibrary();
const gameTools=new GameToolsPanel();
const scryingPane=new ScryingPane();

const app = document.querySelector<HTMLElement>("#app");
if (!app) throw new Error("Missing app root");
const root: HTMLElement = app;
if(TABLE_ART)root.style.setProperty("--table-art",'url("'+TABLE_ART.path+'")');
const abilities = ["str", "dex", "con", "int", "wis", "cha"] as const;
const state = {
  campaigns: [] as CampaignSummary[],
  campaign: null as CampaignSummary | null,
  characters: [] as CharacterSummary[],
  events: [] as CampaignEvent[],
  latestEventSeq:0,
  selectedId: null as string | null,
  busy: false,
  dirty: false,
  view: "character" as WorkspaceView,
  compendiumSection:"documents" as CompendiumSection,
  message: "",
  messageKind: "info"
};
const parse = <T,>(value: string, fallback: T): T => {
  try { return JSON.parse(value) as T; } catch { return fallback; }
};
const date = (value: string) => new Date(value).toLocaleString();
const selected = () => state.characters.find(c => c.id === state.selectedId);
const field = (id: string) => root.querySelector<HTMLInputElement | HTMLTextAreaElement>("#" + id)!;
const lines = (value: unknown[]) => value.map(v => typeof v === "string" ? v : JSON.stringify(v)).join("\n");
const listValue = (id: string) => field(id).value.split("\n").map(v => v.trim()).filter(Boolean);
const allowNavigation = () => {if(dm.pending||assetWorkshop.pending)return false;const dirty=state.dirty||tabletop.dirty||rules.dirty||knowledge.dirty||artLibrary.dirty||assetWorkshop.dirty;if(dirty&&!window.confirm("Discard the unsaved changes?"))return false;if(dirty){artLibrary.discard();assetWorkshop.discard();}return true;};
const status = (message: string, kind = "info") => {
  state.message = message; state.messageKind = kind;
  const region = root.querySelector<HTMLElement>("#status");
  if (region) { region.textContent = message; region.dataset.kind = kind; }
};

async function action(work: () => Promise<void>) {
  if (state.busy) return;
  state.busy = true;
  root.setAttribute("aria-busy", "true");
  const buttons = [...root.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
  const fields = [...root.querySelectorAll<HTMLFieldSetElement>("fieldset:not(:disabled)")];
  buttons.forEach(b => { b.disabled = true; });
  fields.forEach(f => { f.disabled = true; });
  try { await work(); }
  catch (error) { status(String(error), "error"); }
  finally {
    state.busy = false;
    root.setAttribute("aria-busy", "false");
    buttons.forEach(b => { if (b.isConnected) b.disabled = false; });
    fields.forEach(f => { if (f.isConnected) f.disabled = false; });
  }
}

async function snapshot(campaign: CampaignSummary, preserveTabletop = false) {
  const [characters, events, campaigns, board, rolls, scenes, images, rulesState, dmHistory, dmStatus, knowledgeSources, generatedAssets, artEntries, artStatus,toolState,bookmarks] = await Promise.all([
    invoke<CharacterSummary[]>("load_characters", { campaignId: campaign.id }),
    invoke<CampaignEvent[]>("recent_events", { campaignId: campaign.id }),
    invoke<CampaignSummary[]>("list_campaigns"),
    invoke<TabletopState>("load_tabletop", { campaignId: campaign.id }),
    invoke<DiceRoll[]>("load_dice_rolls", { campaignId: campaign.id }),
    invoke<SceneSummary[]>("list_tabletop_scenes", { campaignId: campaign.id }),
    invoke<SceneImage[]>("list_scene_images", { campaignId: campaign.id }),
    invoke<RulesState>("load_rules_settings", {campaignId:campaign.id}),
    invoke<DmHistory>("load_dm_history", {campaignId:campaign.id}),
    invoke<DmRuntimeStatus>("dm_runtime_status").catch(() => null),
    invoke<KnowledgeSource[]>("list_knowledge_sources",{campaignId:campaign.id}),
    invoke<GeneratedAsset[]>("list_generated_assets",{campaignId:campaign.id}),
    invoke<ArtEntry[]>("list_art",{campaignId:campaign.id}),
    invoke<ArtRuntimeStatus>("art_runtime_status").catch(()=>null),
    invoke<GameTools>("load_game_tools",{campaignId:campaign.id}),
    invoke<ScryingBookmark[]>("list_scrying_bookmarks",{campaignId:campaign.id})
  ]);
  state.campaign = campaigns.find(c => c.id === campaign.id) ?? campaign;
  state.campaigns = campaigns;
  state.characters = characters;
  state.events = events;
  state.latestEventSeq=dmHistory.latestEventSeq;
  tabletop.load(board,scenes,images,preserveTabletop);
  dice.load(campaign.id, rolls);
  rules.load(rulesState,preserveTabletop);
  dm.load(campaign.id,dmHistory,dmStatus,rolls);
  knowledge.load(campaign.id,knowledgeSources);
  assetWorkshop.load(campaign.id,generatedAssets,artStatus);
  artLibrary.load(campaign.id,artEntries);
  gameTools.load(campaign.id,toolState,rulesState);
  scryingPane.load(campaign.id,bookmarks);
  historyPanel.load(campaign.id);
  if (!characters.some(c => c.id === state.selectedId)) state.selectedId = characters[0]?.id ?? null;
  state.dirty = false;
}

async function openCampaign(slug: string) {
  const result = await invoke<Saved<CampaignSummary>>("load_campaign", { slug });
  await snapshot(result.value);
  state.message = result.warning ?? "Campaign loaded from this device.";
  state.messageKind = result.warning ? "warning" : "info";
  render();
}

// Keep a committed mutation visible even if the following read fails.
async function afterSave(campaign: CampaignSummary, message: string, warning: string | null, preserveTabletop = false) {
  state.campaign = campaign;
  state.dirty = false;
  state.message = warning ?? message;
  state.messageKind = warning ? "warning" : "success";
  try { await snapshot(campaign,preserveTabletop); }
  catch (error) {
    state.message += " Saved successfully, but refreshing the campaign failed: " + String(error);
    state.messageKind = "warning";
  }
  render();
}

function sheet(character: CharacterSummary): string {
  const stats = parse<Record<string, unknown>>(character.statsJson, {});
  const progression = parse<Record<string, unknown>>(character.progressionJson, {});
  const inventory = parse<unknown[]>(character.inventoryJson, []);
  const conditions = parse<unknown[]>(character.conditionsJson, []);
  return `
    <form id="sheetForm">
      <div class="section-heading"><div><p class="eyebrow">Character sheet</p><h2>${escape(character.name)}</h2></div>
        <button class="primary" type="submit">Save character</button></div>
      <p class="subtle">Record your table’s values. Class features and derived statistics are entered manually.</p>
      <fieldset class="sheet-fields"><legend class="sr-only">Character details</legend>
        <div class="identity">
          <label>Name<input id="sheetName" required maxlength="120" value="${escape(character.name)}"></label>
          <label>Class<input id="sheetClass" required maxlength="120" value="${escape(character.className)}"></label>
          <label>Level<input id="sheetLevel" type="number" required min="1" max="20" value="${character.level}"></label>
        </div>
        <div class="vitals">
          <label>Current HP<input id="hp" type="number" required min="0" max="9999" value="${escape(stats.hp ?? 10)}"></label>
          <label>Maximum HP<input id="maxHp" type="number" required min="1" max="9999" value="${escape(stats.maxHp ?? 10)}"></label>
          <label>Armor class<input id="armorClass" type="number" required min="0" max="99" value="${escape(stats.armorClass ?? 10)}"></label>
        </div>
        <div class="abilities">${abilities.map(key => `<label>${key.toUpperCase()}<input id="${key}" type="number" required min="1" max="30" value="${escape(stats[key] ?? 10)}"></label>`).join("")}</div>
        <p class="subtle">New sheets start with placeholder values of 10. Set them to your character’s actual values before saving.</p>
        <div class="two-columns">
          <label>Inventory <span class="hint">One item per line</span><textarea id="inventory" rows="6">${escape(lines(inventory))}</textarea></label>
          <label>Conditions <span class="hint">One condition per line</span><textarea id="conditions" rows="6">${escape(lines(conditions))}</textarea></label>
        </div>
        <label>Character notes<textarea id="notes" rows="4">${escape(progression.notes ?? "")}</textarea></label>
      </fieldset>
    </form>`;
}

function render() {
  gameTools.detach();
  dice.detach();
  if(state.view==="scrying")scryingPane.detach();else scryingPane.hide();
  root.classList.toggle("tabletop-active",state.view==="tabletop");
  root.classList.toggle("tabletop-focus",state.view==="tabletop"&&tabletop.focused);
  if(state.campaign)compendium.selectCampaign(state.campaign.id);
  const character = selected();
  root.innerHTML = `
    <header class="app-header"><div class="brand-mark" aria-hidden="true">D</div>
      <div><p class="eyebrow">Your world. Your table.</p><h1>Dungeon World AI</h1></div>
      <span class="offline-badge">${isTauri() ? "Saved on this device" : "Desktop preview"}</span></header>
    <div id="status" role="status" aria-live="polite" data-kind="${escape(state.messageKind)}">${escape(state.message)}</div>
    <div class="workspace">
      <aside class="campaign-panel"><div class="section-heading"><h2>Campaigns</h2><button id="refresh" class="quiet">Refresh</button></div>
        <nav aria-label="Saved campaigns" class="campaign-list">${state.campaigns.length ? state.campaigns.map(c => `
          <button class="campaign-card ${c.id === state.campaign?.id ? "active" : ""}" data-slug="${escape(c.slug)}" ${c.id === state.campaign?.id ? 'aria-current="true"' : ""}>
            <strong>${escape(c.name)}</strong><span>Last saved ${escape(date(c.updatedAt))}</span></button>`).join("") : '<p class="empty-note">No campaigns yet. Begin a new adventure below.</p>'}
        </nav>
        <form id="campaignForm"><fieldset><legend>Begin an adventure</legend>
          <label>Campaign name<input id="campaignName" required maxlength="120" placeholder="e.g. Aerathis"></label>
          <button class="primary" type="submit">Create campaign</button></fieldset></form>
      </aside>
      <section class="campaign-workspace" aria-label="Current campaign">
        ${state.campaign ? `
          <div class="campaign-heading"><p class="eyebrow">Campaign workspace · ${escape(rules.label())}</p><h2>${escape(state.campaign.name)}</h2>
            <p class="subtle">${state.characters.length} character${state.characters.length === 1 ? "" : "s"} · Campaign and character changes are saved locally.</p></div>
          <div class="workspace-tabs" aria-label="Campaign views">${WORKSPACE_TABS.map(([view,label])=>`<button data-workspace-view="${view}" aria-pressed="${state.view===view}">${escape(label)}</button>`).join("")}</div>
          ${state.view === "tools" ? gameTools.html(state.characters) : state.view === "scrying" ? scryingPane.html() : state.view === "dm" ? dm.html(state.characters) : state.view === "tabletop" ? tabletop.html(state.characters,dice.html(state.characters)) : state.view === "dice" ? dice.html(state.characters) : state.view === "art" ? assetWorkshop.html()+artLibrary.html() : state.view === "rules" ? rules.html() : state.view === "compendium" ? compendiumWorkspace(state.compendiumSection,knowledge.html(),compendium.html()) : `
          <div class="party-tabs" aria-label="Characters">${state.characters.map(c => `
            <button class="${c.id === state.selectedId ? "active" : ""}" data-character="${escape(c.id)}" aria-pressed="${c.id === state.selectedId}">${escape(c.name)}<span>Level ${c.level} ${escape(c.className)}</span></button>`).join("")}</div>
          <section class="sheet-panel">${character ? sheet(character) : '<div class="empty-state"><p class="eyebrow">The party begins here</p><h2>Add your first character</h2><p>Create a character below, then fill in their sheet.</p></div>'}</section>
          <form id="characterForm" class="new-character"><fieldset><legend>Add a character</legend>
            <label>Name<input id="characterName" required maxlength="120" placeholder="Character name"></label>
            <label>Class<input id="characterClass" required maxlength="120" placeholder="e.g. Barbarian"></label>
            <button type="submit">Add character</button></fieldset></form>
          `}
          ${historyPanel.html(state.events,eventLabel)}
        ` : '<div class="welcome"><p class="eyebrow">A place for every adventure</p><h2>Open a world.<br>Continue your story.</h2><p>Choose a saved campaign or create one to begin keeping your party, character sheets, and campaign history together.</p><div class="welcome-rule"></div><p class="subtle">Open the tabletop to place your party, reveal a map, and continue your adventure.</p></div>'}
      </section>
    </div>`;

  root.querySelectorAll<HTMLButtonElement>("[data-workspace-view]").forEach(button => button.addEventListener("click", () => {
    if (state.busy || button.dataset.workspaceView === state.view || !allowNavigation()) return;
    tabletop.discard(); rules.discard(); knowledge.discard(); state.dirty = false; state.message = "";
    state.view = button.dataset.workspaceView as typeof state.view;
    render();
  }));
  if(state.campaign&&state.view==="tools"){const campaign=state.campaign;gameTools.bind(root,{campaignId:campaign.id,render,action,onSaved:warning=>afterSave(campaign,"Game clocks saved.",warning),onError:message=>status(message,"error")},state.characters);}
  if(state.campaign&&state.view==="scrying"){const campaign=state.campaign;scryingPane.bind(root,{campaignId:campaign.id,render,action,isBusy:()=>state.busy,onSaved:warning=>afterSave(campaign,"Browser bookmarks saved.",warning)});}
  root.querySelectorAll<HTMLButtonElement>("[data-compendium-section]").forEach(button=>button.addEventListener("click",()=>{
    if(state.busy)return;state.compendiumSection=button.dataset.compendiumSection as CompendiumSection;render();
  }));
  if(state.campaign){const campaign=state.campaign;historyPanel.bind(root,{campaignId:campaign.id,expectedSeq:state.latestEventSeq,render,action,
    isBusy:()=>{if(state.busy||dm.pending||assetWorkshop.pending)return true;if(state.dirty||tabletop.dirty||rules.dirty||knowledge.dirty||artLibrary.dirty||assetWorkshop.dirty){status("Save or discard current edits before editing history.","warning");return true;}return false;},
    onSaved:(warning,message)=>afterSave(campaign,message,warning),onError:message=>status(message,"error")});}
  if(state.campaign&&state.view==="compendium"&&state.compendiumSection==="documents"){const campaign=state.campaign;knowledge.bind(root,{campaignId:campaign.id,render,action,onDirty:()=>status("Unsaved source text.","warning"),onMessage:message=>status(message),onSaved:warning=>{compendium.invalidate();return afterSave(campaign,"Sources saved in Compendium and indexed for the Dungeon Master.",warning);}});}
  if (state.campaign && state.view === "dm") {
    const campaign = state.campaign;
    dm.bind(root, {campaignId:campaign.id,render,onSaved:warning=>afterSave(campaign,"Dungeon Master turn saved.",warning)});
  }
  if (state.campaign && state.view === "tabletop") {
    const campaign = state.campaign;
    tabletop.bind(root, { campaignId: campaign.id, characters:state.characters, action, render, isBusy: () => state.busy,
      onDirty: () => status("Unsaved scene changes.", "warning"),
      onError: message => status(message, "error"),
      onSaved: (warning,message) => afterSave(campaign, message??"Scene saved.", warning) });
  }

  if (state.campaign && (state.view === "dice" || state.view === "tabletop")) {
    const campaign = state.campaign;
    dice.bind(root, { campaignId: campaign.id, action,render, isBusy: () => state.busy,
      onSaved: (roll, warning) => afterSave(campaign, "Roll saved: " + describeRoll(roll), warning,true) });
  }

  if(state.campaign&&state.view==="compendium"&&state.compendiumSection==="entries"){
    const campaign=state.campaign;
    compendium.bind(root,{campaignId:campaign.id,action,render,isActive:()=>state.view==="compendium"&&state.compendiumSection==="entries"&&state.campaign?.id===campaign.id,onError:message=>status(message,"error"),onMessage:message=>status(message),onSaved:warning=>afterSave(campaign,"Reference collections imported.",warning)});
  }
  if(state.campaign&&state.view==="rules"){
    const campaign=state.campaign;
    rules.bind(root,{campaignId:campaign.id,action,render,isActive:()=>state.view==="rules",onDirty:()=>status("Unsaved rules profile.","warning"),onError:message=>status(message,"error"),onSaved:warning=>afterSave(campaign,"Rules profile saved.",warning)});
  }
  if(state.campaign&&state.view==="art"){
    const campaign=state.campaign;
    assetWorkshop.bind(root,{campaignId:campaign.id,action,render,onError:message=>status(message,"error"),onSaved:warning=>afterSave(campaign,"PNG, SVG, and token recipe saved with this campaign.",warning),onPlace:(id,name)=>{if(!allowNavigation())return;if(tabletop.addSavedToken(id,name)){state.view="tabletop";status("Generated token placed. Save the scene to keep its position.","warning");render();}else status("This token is unavailable or the scene already has 100 pieces.","error");}});
    artLibrary.bind(root,{campaignId:campaign.id,render,action,onError:message=>status(message,"error"),onSaved:warning=>afterSave(campaign,"Artwork and tags saved with this campaign.",warning),onBackground:(id,saved)=>{if(!allowNavigation())return;if(saved?tabletop.useSavedBackground(id):tabletop.useBundledBackground(id)){state.view="tabletop";status("Background selected. Save scene to keep it.","warning");render();}},onToken:(id,saved,name)=>{if(!allowNavigation())return;if(saved?tabletop.addSavedToken(id,name):tabletop.addBundledToken(id)){state.view="tabletop";status("Token placed. Set its game values and save the scene.","warning");render();}else status("This scene already contains 100 pieces.","error");}});
  }
  root.querySelector("#refresh")?.addEventListener("click", () => {
    if (!allowNavigation()) return;
    void action(async () => {
      if (state.campaign) await openCampaign(state.campaign.slug);
      else { state.campaigns = await invoke<CampaignSummary[]>("list_campaigns"); render(); }
    });
  });
  root.querySelectorAll<HTMLButtonElement>("[data-slug]").forEach(button => button.addEventListener("click", () => {
    if (!allowNavigation()) return;
    void action(() => openCampaign(button.dataset.slug!));
  }));
  root.querySelectorAll<HTMLButtonElement>("[data-character]").forEach(button => button.addEventListener("click", () => {
    if (state.busy || !allowNavigation()) return;
    state.selectedId = button.dataset.character!;
    state.dirty = false;
    state.message = "";
    render();
  }));
  root.querySelector("#campaignForm")?.addEventListener("submit", event => {
    event.preventDefault();
    if (!allowNavigation()) return;
    const name = field("campaignName").value.trim();
    void action(async () => {
      const result = await invoke<Saved<CampaignSummary>>("create_campaign", { name });
      state.selectedId = null; state.characters = []; state.events = [];
      tabletop.load({scene:null, revision:0, updatedAt:null}); dice.load(result.value.id, []); state.view="character";
      await afterSave(result.value, "Campaign created.", result.warning);
    });
  });
  root.querySelector("#characterForm")?.addEventListener("submit", event => {
    event.preventDefault();
    if (!state.campaign || !allowNavigation()) return;
    const campaign = state.campaign;
    const name = field("characterName").value.trim();
    const className = field("characterClass").value.trim();
    void action(async () => {
      const result = await invoke<Saved<CharacterSummary>>("create_character", { campaignId: campaign.id, name, className });
      state.characters.push(result.value); state.selectedId = result.value.id;
      await afterSave(campaign, "Character created. Fill in the sheet and save your changes.", result.warning);
    });
  });
  const form = root.querySelector<HTMLFormElement>("#sheetForm");
  form?.addEventListener("input", () => { state.dirty = true; status("Unsaved character changes.", "warning"); });
  form?.addEventListener("submit", event => {
    event.preventDefault();
    const character = selected();
    if (!state.campaign || !character) return;
    const campaign = state.campaign;
    const stats = parse<Record<string, unknown>>(character.statsJson, {});
    for (const key of [...abilities, "hp", "maxHp", "armorClass"]) stats[key] = Number(field(key).value);
    if (Number(stats.hp) > Number(stats.maxHp)) { status("Current HP cannot exceed maximum HP.", "error"); return; }
    const progression = parse<Record<string, unknown>>(character.progressionJson, {});
    progression.notes = field("notes").value;
    // Keep structured inventory/condition entries untouched when their display has not been edited.
    const inventory = field("inventory").value === lines(parse<unknown[]>(character.inventoryJson, []))
      ? character.inventoryJson : JSON.stringify(listValue("inventory"));
    const conditions = field("conditions").value === lines(parse<unknown[]>(character.conditionsJson, []))
      ? character.conditionsJson : JSON.stringify(listValue("conditions"));
    const update = {
      name: field("sheetName").value.trim(), className: field("sheetClass").value.trim(),
      level: Number(field("sheetLevel").value), statsJson: JSON.stringify(stats),
      inventoryJson: inventory, conditionsJson: conditions, progressionJson: JSON.stringify(progression)
    };
    void action(async () => {
      const result = await invoke<Saved<CharacterSummary>>("update_character", { campaignId: campaign.id, characterId: character.id, update });
      state.characters = state.characters.map(c => c.id === result.value.id ? result.value : c);
      await afterSave(campaign, "Character saved.", result.warning);
    });
  });
}

function eventLabel(event: CampaignEvent): string {
  const character = state.characters.find(c => c.id === event.payload.characterId);
  if (event.eventType === "content.imported") return "Reference collection imported: " + event.payload.name;
  if (event.eventType === "dm.turn.saved") return "Dungeon Master turn saved";
  if (event.eventType === "knowledge.imported") return "DM source imported: "+event.payload.title;
  if (event.eventType === "asset.created") return "Visual asset created: "+event.payload.name;
  if (event.eventType === "tools.changed") return "Game clock / timer updated";
  if (event.eventType === "scrying.bookmark.saved") return "Browser bookmark saved: "+event.payload.title;
  if (event.eventType === "scrying.bookmark.removed") return "Browser bookmark removed";
  if (event.eventType === "history.entry.removed") return "History entry removed";
  if (event.eventType === "history.entry.restored") return "History entry restored";
  if (event.eventType === "rules.changed") return "Campaign rules profile changed";
  if (event.eventType === "dice.rolled") return "Dice roll: " + event.payload.expression + " = " + event.payload.total;
  if (event.eventType === "scene.selected") return "Scene selected: " + (event.payload.name??"Scene");
  if (event.eventType === "image.imported") return "Artwork imported: " + (event.payload.name??"Image");
  if (event.eventType === "scene.saved") return "Scene saved";
  if (event.eventType === "campaign.created") return "Campaign created";
  if (event.eventType === "character.created") return "Character added: " + (character?.name ?? event.payload.name ?? "Adventurer");
  if (event.eventType === "character.updated") return "Character saved: " + (character?.name ?? "Adventurer");
  return event.eventType;
}

window.addEventListener("beforeunload", event => {
  if (state.dirty || tabletop.dirty || rules.dirty || knowledge.dirty || dm.pending || dm.hasDraft || assetWorkshop.pending || assetWorkshop.dirty || artLibrary.dirty) { event.preventDefault(); event.returnValue = ""; }
});
installClipboard(root,message=>status(message,"warning"));
render();
if (isTauri()) {
  void action(async () => { state.campaigns = await invoke<CampaignSummary[]>("list_campaigns"); render(); });
} else {
  status("Open the desktop app to create or load campaigns. This browser view previews the interface.", "info");
}
