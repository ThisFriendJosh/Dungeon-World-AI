#![cfg_attr(all(target_os = "windows", not(test)), windows_subsystem = "windows")]

use std::path::PathBuf;
mod art_runtime;
mod clipboard;
mod reference_documents;
mod scrying;

use dwa_engine::{
    CampaignEvent, CampaignStore, CampaignSummary, CharacterSummary, CharacterUpdate, FariScene,
    Saved, TabletopState,
};
use dwa_engine::{DiceRequest, DiceRoll, ImageImport, SceneImage, SceneSummary};
use dwa_engine::{DmHistory, DmTurn, DmTurnRequest};
use dwa_engine::{RulesCalculation, RulesConfig, RulesQuery, RulesState};
use dwa_local_dm::{LocalDm, RuntimeConfig, RuntimeStatus, PROMPT_VERSION};
use tauri::{Manager, State};

#[tauri::command]
fn open_beta_setup() -> Result<(), String> {
    let setup = std::env::var_os("DWA_BETA_SETUP")
        .map(PathBuf::from)
        .filter(|path| path.is_file() && path.file_name().is_some_and(|n| n == "DungeonWorldAI.exe"))
        .ok_or("Open the beta installer to add local AI. Development builds can use the setup commands in the project folder.")?;
    let mut command = std::process::Command::new(setup);
    command.arg("/configure");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .spawn()
        .map_err(|e| format!("Could not open AI setup: {e}"))?;
    Ok(())
}

#[tauri::command]
fn load_game_tools(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<dwa_engine::GameTools, String> {
    store
        .load_game_tools(&campaign_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn change_game_tools(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    request: dwa_engine::ToolMutation,
) -> Result<Saved<dwa_engine::GameTools>, String> {
    store
        .change_game_tools(&campaign_id, request)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn calculate_game_math(expression: String) -> Result<f64, String> {
    dwa_engine::calculate_game_math(&expression).map_err(|e| e.to_string())
}
#[tauri::command]
fn calculate_damage(query: dwa_engine::DamageQuery) -> Result<dwa_engine::DamageResult, String> {
    dwa_engine::calculate_damage(query).map_err(|e| e.to_string())
}
#[tauri::command]
fn cef_browser_status(
    runtime: State<'_, scrying::ScryingRuntime>,
) -> Result<scrying::ScryingStatus, String> {
    runtime.status()
}
#[tauri::command]
async fn cef_browser_open(
    app: tauri::AppHandle,
    url: String,
    bounds: scrying::PaneBounds,
) -> Result<(), String> {
    #[cfg(windows)]
    let parent = app
        .get_webview_window("main")
        .ok_or("Main window unavailable.")?
        .hwnd()
        .map_err(|e| e.to_string())?
        .0 as isize;
    #[cfg(not(windows))]
    let parent = 0isize;
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<scrying::ScryingRuntime>()
            .open(parent, &url, bounds)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn cef_browser_layout(
    runtime: State<'_, scrying::ScryingRuntime>,
    bounds: scrying::PaneBounds,
) -> Result<(), String> {
    runtime.layout(bounds)
}
#[tauri::command]
fn cef_browser_command(
    runtime: State<'_, scrying::ScryingRuntime>,
    kind: String,
) -> Result<(), String> {
    runtime.command(&kind)
}
#[tauri::command]
async fn cef_browser_close(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<scrying::ScryingRuntime>().shutdown())
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn list_scrying_bookmarks(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<Vec<dwa_engine::ScryingBookmark>, String> {
    store
        .list_scrying_bookmarks(&campaign_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn save_scrying_bookmark(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    request: dwa_engine::BookmarkRequest,
) -> Result<Saved<dwa_engine::ScryingBookmark>, String> {
    store
        .save_scrying_bookmark(&campaign_id, request)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn remove_scrying_bookmark(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    id: String,
    confirmed: bool,
) -> Result<Saved<()>, String> {
    store
        .remove_scrying_bookmark(&campaign_id, &id, confirmed)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_knowledge_sources(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<Vec<dwa_engine::KnowledgeSource>, String> {
    store
        .list_knowledge_sources(&campaign_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn import_knowledge(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    request: dwa_engine::KnowledgeImport,
    original: Option<dwa_engine::OriginalDocumentImport>,
) -> Result<Saved<dwa_engine::KnowledgeSource>, String> {
    store
        .import_knowledge_with_original(&campaign_id, request, original)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn read_knowledge_document(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    id: String,
    offset: u32,
) -> Result<dwa_engine::KnowledgeDocument, String> {
    store
        .read_knowledge_document(&campaign_id, &id, offset)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn search_knowledge_document(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    id: String,
    query: String,
) -> Result<Vec<dwa_engine::DocumentPassage>, String> {
    store
        .search_knowledge_document(&campaign_id, &id, &query)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn attach_knowledge_original(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    id: String,
    original: dwa_engine::OriginalDocumentImport,
) -> Result<Saved<dwa_engine::OriginalDocument>, String> {
    store
        .attach_knowledge_original(&campaign_id, &id, original)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn export_knowledge_original(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    id: String,
) -> Result<serde_json::Value, String> {
    use base64::Engine;
    let (meta, bytes) = store
        .knowledge_original_bytes(&campaign_id, &id)
        .map_err(|e| e.to_string())?;
    Ok(
        serde_json::json!({"name":meta.name,"mediaType":meta.media_type,"dataBase64":base64::engine::general_purpose::STANDARD.encode(bytes)}),
    )
}
#[tauri::command]
fn calculate_campaign(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    request: dwa_engine::CalculationRequest,
) -> Result<Saved<dwa_engine::CalculationRecord>, String> {
    store
        .calculate_campaign(&campaign_id, request)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn list_calculations(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    offset: u32,
) -> Result<Vec<dwa_engine::CalculationRecord>, String> {
    store
        .list_calculations(&campaign_id, offset)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn search_dm_references(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    query: String,
) -> Result<Vec<dwa_engine::ReferenceHit>, String> {
    store
        .search_dm_references(&campaign_id, &query)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn load_editable_history(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<Vec<dwa_engine::HistoryEntry>, String> {
    store
        .load_editable_history(&campaign_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn edit_history_entry(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    request: dwa_engine::HistoryEdit,
) -> Result<Saved<serde_json::Value>, String> {
    store
        .edit_history_entry(&campaign_id, request)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn preview_token_asset(
    spec: dwa_engine::TokenSpec,
    overlay_png_data_url: Option<String>,
) -> Result<String, String> {
    dwa_engine::token_svg_with_artwork(&spec, overlay_png_data_url.as_deref())
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn art_runtime_status(runtime: State<'_, art_runtime::ArtRuntime>) -> art_runtime::ArtStatus {
    runtime.status()
}
#[tauri::command]
fn cancel_token_art(runtime: State<'_, art_runtime::ArtRuntime>) -> bool {
    runtime.cancel()
}
#[tauri::command]
async fn generate_token_art(
    app: tauri::AppHandle,
    request: art_runtime::ArtRequest,
    progress: tauri::ipc::Channel<String>,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<art_runtime::ArtRuntime>()
            .generate(request, |message| {
                let _ = progress.send(message.into());
            })
    })
    .await
    .map_err(|e| format!("Image worker stopped: {e}"))?
}
#[tauri::command]
fn list_art(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<Vec<dwa_engine::ArtEntry>, String> {
    store.list_art(&campaign_id).map_err(|e| e.to_string())
}
#[tauri::command]
fn import_art(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    request: dwa_engine::ArtImport,
) -> Result<Saved<dwa_engine::ArtEntry>, String> {
    store
        .import_art(&campaign_id, request)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn update_art(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    request: dwa_engine::ArtUpdate,
) -> Result<Saved<dwa_engine::ArtMetadata>, String> {
    store
        .update_art(&campaign_id, request)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn load_art_preview(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    id: String,
) -> Result<String, String> {
    store
        .load_art_preview(&campaign_id, &id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn load_art_svg(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    id: String,
) -> Result<String, String> {
    store
        .load_art_svg(&campaign_id, &id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn save_token_asset(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    request: dwa_engine::TokenAssetRequest,
) -> Result<Saved<dwa_engine::GeneratedAsset>, String> {
    store
        .save_token_asset(&campaign_id, request)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn list_generated_assets(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<Vec<dwa_engine::GeneratedAsset>, String> {
    store
        .list_generated_assets(&campaign_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn load_generated_asset(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    id: String,
) -> Result<serde_json::Value, String> {
    store
        .load_generated_asset(&campaign_id, &id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn load_dm_history(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<DmHistory, String> {
    store
        .load_dm_history(&campaign_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn dm_runtime_status(runtime: State<'_, LocalDm>) -> Result<RuntimeStatus, String> {
    runtime.status()
}

#[tauri::command]
fn cancel_dm_turn(runtime: State<'_, LocalDm>) -> Result<bool, String> {
    runtime.cancel()
}

#[tauri::command]
async fn play_dm_turn(
    app: tauri::AppHandle,
    campaign_id: String,
    request: DmTurnRequest,
) -> Result<Saved<DmTurn>, String> {
    // Model startup and generation never block the webview/main thread.
    tauri::async_runtime::spawn_blocking(move || {
        let store = app.state::<CampaignStore>();
        if let Some(turn) = store
            .find_dm_turn(&campaign_id, &request)
            .map_err(|e| e.to_string())?
        {
            return Ok(Saved {
                value: turn,
                warning: None,
            });
        }
        let context = store
            .prepare_dm_context(&campaign_id, &request)
            .map_err(|e| e.to_string())?;
        app.state::<LocalDm>().run_turn_with_tools(
            &context,
            &request.player_text,
            |tool| {
                store
                    .read_dm_tool(&campaign_id, context.state_seq, tool)
                    .map_err(|e| e.to_string())
            },
            |proposal, model| {
                store
                    .commit_dm_turn(&campaign_id, &request, proposal, model, PROMPT_VERSION)
                    .map_err(|e| e.to_string())
            },
        )
    })
    .await
    .map_err(|e| format!("DM worker stopped: {e}"))?
}

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

#[tauri::command]
fn load_tabletop(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<TabletopState, String> {
    store.load_tabletop(&campaign_id).map_err(|e| e.to_string())
}
#[tauri::command]
fn save_tabletop(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    scene: FariScene,
    expected_revision: i64,
) -> Result<Saved<TabletopState>, String> {
    store
        .save_tabletop(&campaign_id, scene, expected_revision)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn roll_dice(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    request: DiceRequest,
) -> Result<Saved<DiceRoll>, String> {
    store
        .roll_dice(&campaign_id, request)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn load_dice_rolls(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<Vec<DiceRoll>, String> {
    store
        .load_dice_rolls(&campaign_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_tabletop_scenes(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<Vec<SceneSummary>, String> {
    store
        .list_tabletop_scenes(&campaign_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn select_tabletop_scene(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    scene_id: String,
    expected_revision: i64,
) -> Result<Saved<TabletopState>, String> {
    store
        .select_tabletop_scene(&campaign_id, &scene_id, expected_revision)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn list_scene_images(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<Vec<SceneImage>, String> {
    store
        .list_scene_images(&campaign_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn load_scene_image(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    image_id: String,
) -> Result<String, String> {
    store
        .load_scene_image(&campaign_id, &image_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn import_scene_image(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    request: ImageImport,
) -> Result<Saved<SceneImage>, String> {
    store
        .import_scene_image(&campaign_id, request)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn load_rules_settings(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<RulesState, String> {
    store
        .load_rules_settings(&campaign_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn save_rules_settings(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    config: RulesConfig,
    expected_revision: i64,
) -> Result<Saved<RulesState>, String> {
    store
        .save_rules_settings(&campaign_id, config, expected_revision)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn calculate_rules(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    query: RulesQuery,
) -> Result<RulesCalculation, String> {
    store
        .calculate_rules(&campaign_id, query)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn import_content(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    request: dwa_engine::ContentImport,
) -> Result<Saved<dwa_engine::ContentPack>, String> {
    store
        .import_content(&campaign_id, request)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn list_content_packs(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<Vec<dwa_engine::ContentPack>, String> {
    store
        .list_content_packs(&campaign_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn list_content_books(
    store: State<'_, CampaignStore>,
    campaign_id: String,
) -> Result<Vec<String>, String> {
    store
        .list_content_books(&campaign_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn search_content(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    query: dwa_engine::ContentQuery,
) -> Result<dwa_engine::ContentResults, String> {
    store
        .search_content(&campaign_id, query)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn load_content_entry(
    store: State<'_, CampaignStore>,
    campaign_id: String,
    entry_id: String,
) -> Result<dwa_engine::ContentDetail, String> {
    store
        .load_content_entry(&campaign_id, &entry_id)
        .map_err(|e| e.to_string())
}
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = match std::env::var("DWA_DATA_DIR") {
                Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
                _ => app.path().app_data_dir()?,
            };
            app.manage(CampaignStore::new(data_dir.join("campaigns"))?);
            app.manage(LocalDm::new(RuntimeConfig::discover(
                app.path().resource_dir()?,
                data_dir.join("logs"),
            )));
            app.manage(art_runtime::ArtRuntime::discover(data_dir.join("logs")));
            app.manage(scrying::ScryingRuntime::new(
                app.path().resource_dir()?,
                data_dir,
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_beta_setup,
            load_game_tools,
            change_game_tools,
            calculate_game_math,
            calculate_damage,
            cef_browser_status,
            cef_browser_open,
            cef_browser_layout,
            cef_browser_command,
            cef_browser_close,
            list_scrying_bookmarks,
            save_scrying_bookmark,
            remove_scrying_bookmark,
            clipboard::read_clipboard_text,
            clipboard::write_clipboard_text,
            reference_documents::prepare_reference_document,
            list_campaigns,
            create_campaign,
            load_campaign,
            create_character,
            update_character,
            load_characters,
            recent_events,
            load_tabletop,
            save_tabletop,
            roll_dice,
            load_dice_rolls,
            list_tabletop_scenes,
            select_tabletop_scene,
            list_scene_images,
            load_scene_image,
            import_scene_image,
            load_rules_settings,
            save_rules_settings,
            calculate_rules,
            import_content,
            list_content_packs,
            list_content_books,
            search_content,
            load_content_entry,
            load_dm_history,
            dm_runtime_status,
            play_dm_turn,
            cancel_dm_turn,
            list_knowledge_sources,
            import_knowledge,
            read_knowledge_document,
            search_knowledge_document,
            attach_knowledge_original,
            export_knowledge_original,
            reference_documents::render_reference_pdf_page,
            calculate_campaign,
            list_calculations,
            search_dm_references,
            load_editable_history,
            edit_history_entry,
            preview_token_asset,
            save_token_asset,
            list_generated_assets,
            load_generated_asset,
            art_runtime_status,
            generate_token_art,
            cancel_token_art,
            list_art,
            import_art,
            update_art,
            load_art_preview,
            load_art_svg
        ])
        .build(tauri::generate_context!())
        .expect("error while building Dungeon World AI")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<LocalDm>().shutdown();
                app.state::<art_runtime::ArtRuntime>().shutdown();
                app.state::<scrying::ScryingRuntime>().shutdown();
            }
        });
}
