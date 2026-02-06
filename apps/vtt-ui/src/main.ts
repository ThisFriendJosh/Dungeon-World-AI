import { invoke } from "@tauri-apps/api/core";

type CampaignSummary = {
  id: string;
  name: string;
  slug: string;
  createdAt: string;
  updatedAt: string;
};

type CharacterSummary = {
  id: string;
  campaignId: string;
  name: string;
  className: string;
  level: number;
  statsJson: string;
  inventoryJson: string;
  conditionsJson: string;
  progressionJson: string;
  createdAt: string;
  updatedAt: string;
};

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) throw new Error("Missing app root");

const state = {
  campaign: null as CampaignSummary | null,
  characters: [] as CharacterSummary[]
};

const render = () => {
  app.innerHTML = `
    <h1>Dungeon World AI (v0)</h1>

    <section>
      <h2>Create Campaign</h2>
      <input id="campaignName" placeholder="Campaign name" />
      <button id="createCampaign">Create</button>
      <button id="loadCampaign">Load by slug</button>
      <input id="campaignSlug" placeholder="campaign-slug" />
      <pre id="campaignState">${state.campaign ? JSON.stringify(state.campaign, null, 2) : "No campaign loaded"}</pre>
    </section>

    <section>
      <h2>Create Character</h2>
      <input id="characterName" placeholder="Character name" ${!state.campaign ? "disabled" : ""} />
      <input id="characterClass" placeholder="Class" value="Adventurer" ${!state.campaign ? "disabled" : ""} />
      <button id="createCharacter" ${!state.campaign ? "disabled" : ""}>Create</button>
      <button id="refreshCharacters" ${!state.campaign ? "disabled" : ""}>Refresh</button>
      <pre id="characterState">${JSON.stringify(state.characters, null, 2)}</pre>
    </section>
  `;

  document.getElementById("createCampaign")?.addEventListener("click", async () => {
    const name = (document.getElementById("campaignName") as HTMLInputElement).value.trim();
    if (!name) return;
    state.campaign = await invoke<CampaignSummary>("create_campaign", { name });
    state.characters = [];
    render();
  });

  document.getElementById("loadCampaign")?.addEventListener("click", async () => {
    const slug = (document.getElementById("campaignSlug") as HTMLInputElement).value.trim();
    if (!slug) return;
    state.campaign = await invoke<CampaignSummary>("load_campaign", { slug });
    state.characters = await invoke<CharacterSummary[]>("load_characters", { campaignId: state.campaign.id });
    render();
  });

  document.getElementById("createCharacter")?.addEventListener("click", async () => {
    if (!state.campaign) return;
    const name = (document.getElementById("characterName") as HTMLInputElement).value.trim();
    const className = (document.getElementById("characterClass") as HTMLInputElement).value.trim() || "Adventurer";
    if (!name) return;
    await invoke<CharacterSummary>("create_character", { campaignId: state.campaign.id, name, className });
    state.characters = await invoke<CharacterSummary[]>("load_characters", { campaignId: state.campaign.id });
    render();
  });

  document.getElementById("refreshCharacters")?.addEventListener("click", async () => {
    if (!state.campaign) return;
    state.characters = await invoke<CharacterSummary[]>("load_characters", { campaignId: state.campaign.id });
    render();
  });
};

render();
