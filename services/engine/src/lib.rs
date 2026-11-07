use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use chrono::Utc;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

mod tabletop;
pub use tabletop::{FariScene, SceneCard, SceneCards, SceneTextBlock, TabletopState};

const MIGRATION_TABLETOP: &str = include_str!("../migrations/002_tabletop.sql");
const MIGRATION: &str = include_str!("../migrations/001_init.sql");
// Serializes mutations and JSONL mirror repairs within this desktop process.
static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("File error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("{0}")]
    Invalid(String),
    #[error("Campaign not found: {0}")]
    CampaignNotFound(String),
    #[error("Character not found: {0}")]
    CharacterNotFound(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignSummary {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSummary {
    pub id: String,
    pub campaign_id: String,
    pub name: String,
    pub class_name: String,
    pub level: i64,
    pub stats_json: String,
    pub inventory_json: String,
    pub conditions_json: String,
    pub progression_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterUpdate {
    pub name: String,
    pub class_name: String,
    pub level: i64,
    pub stats_json: String,
    pub inventory_json: String,
    pub conditions_json: String,
    pub progression_json: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignEvent {
    pub seq: i64,
    pub event_type: String,
    pub payload: Value,
    pub created_at: String,
}

/// SQLite is canonical. A mirror failure must not make a committed save look unsaved.
#[derive(Debug, Serialize)]
pub struct Saved<T> {
    pub value: T,
    pub warning: Option<String>,
}

pub struct CampaignStore {
    root: PathBuf,
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn validate_name(value: &str, label: &str) -> Result<String, EngineError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 120 {
        return Err(EngineError::Invalid(format!(
            "{label} must contain 1–120 characters."
        )));
    }
    Ok(value.to_string())
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    for ch in input.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !out.ends_with('-') {
            out.push('-');
        }
    }
    let slug = out.trim_matches('-');
    if slug.is_empty() {
        "campaign".to_string()
    } else {
        slug.to_string()
    }
}

fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 160
        && slug
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        && !slug.starts_with('-')
        && !slug.ends_with('-')
}

fn campaign_row(row: &Row<'_>) -> rusqlite::Result<CampaignSummary> {
    Ok(CampaignSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        slug: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn character_row(row: &Row<'_>) -> rusqlite::Result<CharacterSummary> {
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
}

const CHARACTER_COLUMNS: &str = "id, campaign_id, name, class_name, level, stats_json, inventory_json, conditions_json, progression_json, created_at, updated_at";

fn connect(path: &Path, create: bool) -> Result<Connection, EngineError> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX
        | if create {
            OpenFlags::SQLITE_OPEN_CREATE
        } else {
            OpenFlags::empty()
        };
    let conn = Connection::open_with_flags(path.join("campaign.sqlite"), flags)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    if create {
        conn.execute_batch(MIGRATION)?;
    }
    conn.execute_batch(MIGRATION_TABLETOP)?;
    Ok(conn)
}

fn record_event(
    tx: &Transaction<'_>,
    campaign_id: &str,
    kind: &str,
    payload: &Value,
    ts: &str,
) -> Result<(), EngineError> {
    tx.execute(
        "INSERT INTO events (campaign_id, event_type, payload_json, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![campaign_id, kind, payload.to_string(), ts],
    )?;
    tx.execute(
        "UPDATE campaigns SET updated_at = ?1 WHERE id = ?2",
        params![ts, campaign_id],
    )?;
    Ok(())
}

fn event_rows(
    conn: &Connection,
    campaign_id: &str,
    limit: Option<i64>,
) -> Result<Vec<CampaignEvent>, EngineError> {
    let mut stmt = conn.prepare(
        "SELECT seq, event_type, payload_json, created_at FROM events WHERE campaign_id = ?1 ORDER BY seq DESC LIMIT ?2"
    )?;
    let rows = stmt.query_map(params![campaign_id, limit.unwrap_or(-1)], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    let mut events = Vec::new();
    for row in rows {
        let (seq, event_type, payload, created_at) = row?;
        events.push(CampaignEvent {
            seq,
            event_type,
            created_at,
            payload: serde_json::from_str(&payload)
                .map_err(|_| EngineError::Invalid(format!("Event {seq} contains invalid JSON.")))?,
        });
    }
    Ok(events)
}

fn repair_mirror(dir: &Path, conn: &Connection, campaign_id: &str) -> Result<(), EngineError> {
    let temp = dir.join(format!(".events-{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut events = event_rows(conn, campaign_id, None)?;
        events.reverse();
        let mut file = BufWriter::new(
            File::create(&temp)
                .map_err(|e| EngineError::Invalid(format!("Create event mirror: {e}")))?,
        );
        for event in events {
            writeln!(
                file,
                "{}",
                serde_json::to_string(&event).expect("event is serializable")
            )?;
        }
        file.flush()?;
        file.get_ref().sync_all()?;
        drop(file);
        fs::rename(&temp, dir.join("events.log"))
            .map_err(|e| EngineError::Invalid(format!("Replace event mirror: {e}")))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn saved<T>(value: T, dir: &Path, conn: &Connection, campaign_id: &str) -> Saved<T> {
    Saved {
        value,
        warning: repair_mirror(dir, conn, campaign_id).err().map(|err| {
            format!("Saved in the database. The event log could not be refreshed: {err}. Reopen the campaign to retry.")
        }),
    }
}

impl CampaignStore {
    /// root is the campaigns directory, not an individual campaign directory.
    pub fn new(root: PathBuf) -> Result<Self, EngineError> {
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    fn directory(&self, slug: &str) -> Result<PathBuf, EngineError> {
        if !valid_slug(slug) {
            return Err(EngineError::Invalid("Invalid campaign identifier.".into()));
        }
        let dir = self.root.join(slug);
        let metadata = fs::symlink_metadata(&dir).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                EngineError::CampaignNotFound(slug.into())
            } else {
                EngineError::Io(e)
            }
        })?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || !dir.join("campaign.sqlite").is_file()
        {
            return Err(EngineError::CampaignNotFound(slug.into()));
        }
        // Also rejects Windows junctions or other links escaping the campaign root.
        if !dir.canonicalize()?.starts_with(self.root.canonicalize()?) {
            return Err(EngineError::Invalid(
                "Campaign directory is outside the data folder.".into(),
            ));
        }
        Ok(dir)
    }

    pub fn list_campaigns(&self) -> Result<Vec<CampaignSummary>, EngineError> {
        let mut campaigns = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let slug = entry.file_name().to_string_lossy().into_owned();
            if !valid_slug(&slug) || !entry.path().join("campaign.sqlite").is_file() {
                continue;
            }
            let dir = self.directory(&slug)?;
            let conn = connect(&dir, false)?;
            let campaign = conn
                .query_row(
                    "SELECT id, name, slug, created_at, updated_at FROM campaigns WHERE slug = ?1",
                    [&slug],
                    campaign_row,
                )
                .optional()?;
            if let Some(campaign) = campaign {
                campaigns.push(campaign);
            }
        }
        campaigns.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(a.name.cmp(&b.name)));
        Ok(campaigns)
    }

    fn locate(&self, campaign_id: &str) -> Result<PathBuf, EngineError> {
        let campaign = self
            .list_campaigns()?
            .into_iter()
            .find(|c| c.id == campaign_id)
            .ok_or_else(|| EngineError::CampaignNotFound(campaign_id.into()))?;
        self.directory(&campaign.slug)
    }

    pub fn create_campaign(&self, name: &str) -> Result<Saved<CampaignSummary>, EngineError> {
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let name = validate_name(name, "Campaign name")?;
        let id = Uuid::new_v4().to_string();
        let mut slug = slugify(&name);
        let mut dir = self.root.join(&slug);
        match fs::create_dir(&dir) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                slug = format!("{slug}-{}", &id[..8]);
                dir = self.root.join(&slug);
                fs::create_dir(&dir)?;
            }
            Err(err) => return Err(err.into()),
        }
        let mut conn = connect(&dir, true)?;
        let timestamp = now();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO campaigns (id, name, slug, master_seed, art_profile_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, '{}', ?5, ?5)",
            params![id, name, slug, 42_i64, timestamp],
        )?;
        record_event(
            &tx,
            &id,
            "campaign.created",
            &serde_json::json!({"campaignId": id, "name": name, "slug": slug}),
            &timestamp,
        )?;
        tx.commit()?;
        let campaign = CampaignSummary {
            id: id.clone(),
            name,
            slug,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        Ok(saved(campaign, &dir, &conn, &id))
    }

    pub fn load_campaign(&self, slug: &str) -> Result<Saved<CampaignSummary>, EngineError> {
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.directory(slug)?;
        let conn = connect(&dir, false)?;
        let campaign = conn
            .query_row(
                "SELECT id, name, slug, created_at, updated_at FROM campaigns WHERE slug = ?1",
                [slug],
                campaign_row,
            )
            .optional()?
            .ok_or_else(|| EngineError::CampaignNotFound(slug.into()))?;
        let id = campaign.id.clone();
        Ok(saved(campaign, &dir, &conn, &id))
    }

    pub fn load_characters(&self, campaign_id: &str) -> Result<Vec<CharacterSummary>, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let mut stmt = conn.prepare(&format!("SELECT {CHARACTER_COLUMNS} FROM characters WHERE campaign_id = ?1 ORDER BY created_at, id"))?;
        let rows = stmt.query_map([campaign_id], character_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn recent_events(&self, campaign_id: &str) -> Result<Vec<CampaignEvent>, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        event_rows(&conn, campaign_id, Some(50))
    }

    pub fn create_character(
        &self,
        campaign_id: &str,
        name: &str,
        class_name: &str,
    ) -> Result<Saved<CharacterSummary>, EngineError> {
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let name = validate_name(name, "Character name")?;
        let class_name = validate_name(class_name, "Class")?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let id = Uuid::new_v4().to_string();
        let timestamp = now();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO characters (id, campaign_id, name, class_name, level, stats_json, inventory_json, conditions_json, progression_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 1, '{}', '[]', '[]', '{}', ?5, ?5)",
            params![id, campaign_id, name, class_name, timestamp],
        )?;
        record_event(
            &tx,
            campaign_id,
            "character.created",
            &serde_json::json!({"campaignId": campaign_id, "characterId": id, "name": name, "className": class_name}),
            &timestamp,
        )?;
        tx.commit()?;
        let character = conn.query_row(
            &format!("SELECT {CHARACTER_COLUMNS} FROM characters WHERE id = ?1"),
            [&id],
            character_row,
        )?;
        Ok(saved(character, &dir, &conn, campaign_id))
    }

    pub fn update_character(
        &self,
        campaign_id: &str,
        character_id: &str,
        mut update: CharacterUpdate,
    ) -> Result<Saved<CharacterSummary>, EngineError> {
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        update.name = validate_name(&update.name, "Character name")?;
        update.class_name = validate_name(&update.class_name, "Class")?;
        validate_update(&update)?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let timestamp = now();
        let tx = conn.transaction()?;
        let changed = tx.execute(
            "UPDATE characters SET name = ?1, class_name = ?2, level = ?3, stats_json = ?4, inventory_json = ?5, conditions_json = ?6, progression_json = ?7, updated_at = ?8 WHERE id = ?9 AND campaign_id = ?10",
            params![update.name, update.class_name, update.level, update.stats_json, update.inventory_json,
                update.conditions_json, update.progression_json, timestamp, character_id, campaign_id],
        )?;
        if changed != 1 {
            return Err(EngineError::CharacterNotFound(character_id.into()));
        }
        record_event(
            &tx,
            campaign_id,
            "character.updated",
            &serde_json::json!({"campaignId": campaign_id, "characterId": character_id, "character": update}),
            &timestamp,
        )?;
        tx.commit()?;
        let character = conn.query_row(
            &format!(
                "SELECT {CHARACTER_COLUMNS} FROM characters WHERE id = ?1 AND campaign_id = ?2"
            ),
            params![character_id, campaign_id],
            character_row,
        )?;
        Ok(saved(character, &dir, &conn, campaign_id))
    }
}

fn validate_update(update: &CharacterUpdate) -> Result<(), EngineError> {
    if !(1..=20).contains(&update.level) {
        return Err(EngineError::Invalid(
            "Level must be between 1 and 20.".into(),
        ));
    }
    for (label, text, object) in [
        ("Statistics", &update.stats_json, true),
        ("Inventory", &update.inventory_json, false),
        ("Conditions", &update.conditions_json, false),
        ("Progression", &update.progression_json, true),
    ] {
        if text.len() > 100_000 {
            return Err(EngineError::Invalid(format!("{label} is too large.")));
        }
        let value: Value = serde_json::from_str(text)
            .map_err(|_| EngineError::Invalid(format!("{label} contains invalid data.")))?;
        if (object && !value.is_object()) || (!object && !value.is_array()) {
            return Err(EngineError::Invalid(format!(
                "{label} has the wrong data shape."
            )));
        }
        if label == "Statistics" {
            for (key, low, high) in [
                ("str", 1, 30),
                ("dex", 1, 30),
                ("con", 1, 30),
                ("int", 1, 30),
                ("wis", 1, 30),
                ("cha", 1, 30),
                ("hp", 0, 9999),
                ("maxHp", 1, 9999),
                ("armorClass", 0, 99),
            ] {
                if let Some(number) = value.get(key) {
                    if !number.as_i64().is_some_and(|n| (low..=high).contains(&n)) {
                        return Err(EngineError::Invalid(format!("Invalid value for {key}.")));
                    }
                }
            }
            if let (Some(hp), Some(max_hp)) = (value["hp"].as_i64(), value["maxHp"].as_i64()) {
                if hp > max_hp {
                    return Err(EngineError::Invalid(
                        "Current HP cannot exceed maximum HP.".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
