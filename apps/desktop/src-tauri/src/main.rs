use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::Manager;
use thiserror::Error;
use uuid::Uuid;

const MIGRATION_001: &str = include_str!("../../../../services/engine/migrations/001_init.sql");

#[derive(Debug, Error)]
enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("DB error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Campaign not found: {0}")]
    CampaignNotFound(String),
    #[error("Tauri error: {0}")]
    Tauri(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CampaignSummary {
    id: String,
    name: String,
    slug: String,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CharacterSummary {
    id: String,
    campaign_id: String,
    name: String,
    class_name: String,
    level: i64,
    stats_json: String,
    inventory_json: String,
    conditions_json: String,
    progression_json: String,
    created_at: String,
    updated_at: String,
}

fn campaigns_root(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    if let Ok(override_dir) = std::env::var("DWA_DATA_DIR") {
        if !override_dir.trim().is_empty() {
            let path = PathBuf::from(override_dir).join("campaigns");
            fs::create_dir_all(&path)?;
            return Ok(path);
        }
    }

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Tauri(e.to_string()))?;
    let campaigns = app_data.join("campaigns");
    fs::create_dir_all(&campaigns)?;
    Ok(campaigns)
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    for ch in input.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if (ch.is_ascii_whitespace() || ch == '-' || ch == '_') && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn open_campaign_db(campaign_dir: &Path) -> Result<Connection, AppError> {
    let db_path = campaign_dir.join("campaign.sqlite");
    let conn = Connection::open(db_path)?;
    conn.execute_batch(MIGRATION_001)?;
    Ok(conn)
}

fn append_event_log(
    campaign_dir: &Path,
    event_type: &str,
    payload_json: &str,
) -> Result<(), AppError> {
    let log_path = campaign_dir.join("events.log");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let record = serde_json::json!({
        "ts": now(),
        "eventType": event_type,
        "payload": serde_json::from_str::<serde_json::Value>(payload_json).unwrap_or(serde_json::Value::String(payload_json.to_string()))
    });
    writeln!(file, "{}", record)?;
    Ok(())
}

#[tauri::command]
fn create_campaign(app: tauri::AppHandle, name: String) -> Result<CampaignSummary, AppError> {
    let slug = slugify(&name);
    if slug.is_empty() {
        return Err(AppError::CampaignNotFound(
            "Invalid campaign name".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let timestamp = now();
    let root = campaigns_root(&app)?;
    let campaign_dir = root.join(&slug);
    fs::create_dir_all(&campaign_dir)?;
    let conn = open_campaign_db(&campaign_dir)?;

    conn.execute(
        "INSERT OR REPLACE INTO campaigns (id, name, slug, master_seed, art_profile_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, name, slug, 42_i64, "{}", timestamp, timestamp],
    )?;

    let payload = serde_json::json!({"campaignId": id, "name": name, "slug": slug}).to_string();
    conn.execute(
        "INSERT INTO events (campaign_id, event_type, payload_json, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, "campaign.created", payload, now()],
    )?;
    append_event_log(&campaign_dir, "campaign.created", &payload)?;

    Ok(CampaignSummary {
        id,
        name,
        slug,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    })
}

#[tauri::command]
fn load_campaign(app: tauri::AppHandle, slug: String) -> Result<CampaignSummary, AppError> {
    let root = campaigns_root(&app)?;
    let campaign_dir = root.join(&slug);
    let conn = open_campaign_db(&campaign_dir)?;
    let campaign = conn.query_row(
        "SELECT id, name, slug, created_at, updated_at FROM campaigns ORDER BY created_at DESC LIMIT 1",
        [],
        |row| {
            Ok(CampaignSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                slug: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    )?;
    Ok(campaign)
}

#[tauri::command]
fn create_character(
    app: tauri::AppHandle,
    campaign_id: String,
    name: String,
    class_name: String,
) -> Result<CharacterSummary, AppError> {
    let root = campaigns_root(&app)?;
    let slug = find_campaign_slug_by_id(&root, &campaign_id)?;
    let campaign_dir = root.join(slug);
    let conn = open_campaign_db(&campaign_dir)?;

    let id = Uuid::new_v4().to_string();
    let timestamp = now();
    conn.execute(
        "INSERT INTO characters (id, campaign_id, name, class_name, level, stats_json, inventory_json, conditions_json, progression_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 1, '{}', '[]', '[]', '{}', ?5, ?6)",
        params![id, campaign_id, name, class_name, timestamp, timestamp],
    )?;

    let payload = serde_json::json!({"campaignId": campaign_id, "characterId": id, "name": name, "className": class_name}).to_string();
    conn.execute(
        "INSERT INTO events (campaign_id, event_type, payload_json, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![campaign_id, "character.created", payload, now()],
    )?;
    append_event_log(&campaign_dir, "character.created", &payload)?;

    Ok(CharacterSummary {
        id,
        campaign_id,
        name,
        class_name,
        level: 1,
        stats_json: "{}".to_string(),
        inventory_json: "[]".to_string(),
        conditions_json: "[]".to_string(),
        progression_json: "{}".to_string(),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    })
}

#[tauri::command]
fn load_characters(
    app: tauri::AppHandle,
    campaign_id: String,
) -> Result<Vec<CharacterSummary>, AppError> {
    let root = campaigns_root(&app)?;
    let slug = find_campaign_slug_by_id(&root, &campaign_id)?;
    let campaign_dir = root.join(slug);
    let conn = open_campaign_db(&campaign_dir)?;
    let mut stmt = conn.prepare("SELECT id, campaign_id, name, class_name, level, stats_json, inventory_json, conditions_json, progression_json, created_at, updated_at FROM characters WHERE campaign_id = ?1 ORDER BY created_at ASC")?;
    let rows = stmt.query_map([campaign_id], |row| {
        Ok(CharacterSummary {
            id: row.get(0)?,
            campaign_id: row.get(1)?,
            name: row.get(2)?,
            class_name: row.get(3)?,
            level: row.get(4)?,
            stats_json: row.get(5)?,
            inventory_json: row.get(6)?,
            conditions_json: row.get(7)?,
            progression_json: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    })?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn find_campaign_slug_by_id(root: &Path, campaign_id: &str) -> Result<String, AppError> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }
        let db_path = entry.path().join("campaign.sqlite");
        if !db_path.exists() {
            continue;
        }
        let conn = Connection::open(db_path)?;
        let mut stmt = conn.prepare("SELECT slug FROM campaigns WHERE id = ?1 LIMIT 1")?;
        let slug = stmt.query_row([campaign_id], |row| row.get::<_, String>(0));
        if let Ok(slug) = slug {
            return Ok(slug);
        }
    }
    Err(AppError::CampaignNotFound(campaign_id.to_string()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            create_campaign,
            load_campaign,
            create_character,
            load_characters
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}
