use std::path::PathBuf;

use dwa_engine::{
    CampaignEvent, CampaignStore, CampaignSummary, CharacterSummary, CharacterUpdate, Saved,
};
use tauri::{Manager, State};

#[tauri::command]
fn list_campaigns(store: State<'_, CampaignStore>) -> Result<Vec<CampaignSummary>, String> {
    store.list_campaigns().map_err(|e| e.to_string())
}

#[tauri::command]
fn create_campaign(
    store: State<'_, CampaignStore>,
    name: String,
) -> Result<Saved<CampaignSummary>, String> {
    store.create_campaign(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_campaign(
    store: State<'_, CampaignStore>,
    slug: String,
) -> Result<Saved<CampaignSummary>, String> {
    store.load_campaign(&slug).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_character(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    name: String,
    class_name: String,
) -> Result<Saved<CharacterSummary>, String> {
    store
        .create_character(&campaign_id, &name, &class_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_character(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    character_id: String,
    update: CharacterUpdate,
) -> Result<Saved<CharacterSummary>, String> {
    store
        .update_character(&campaign_id, &character_id, update)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn load_characters(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<Vec<CharacterSummary>, String> {
    store
        .load_characters(&campaign_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn recent_events(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<Vec<CampaignEvent>, String> {
    store.recent_events(&campaign_id).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = match std::env::var("DWA_DATA_DIR") {
                Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
                _ => app.path().app_data_dir()?,
            };
            app.manage(CampaignStore::new(data_dir.join("campaigns"))?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_campaigns,
            create_campaign,
            load_campaign,
            create_character,
            update_character,
            load_characters,
            recent_events
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dungeon World AI");
}
