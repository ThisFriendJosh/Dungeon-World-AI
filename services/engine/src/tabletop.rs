use super::*;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneTextBlock {
    pub id: String,
    #[serde(rename = "type")]
    pub block_type: String,
    pub label: String,
    pub value: String,
    pub meta: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneCard {
    pub id: String,
    pub title_label: String,
    pub title: String,
    pub color: String,
    pub played_during_turn: bool,
    pub pinned: bool,
    pub blocks: Vec<SceneTextBlock>,
    pub sub_cards: Vec<Value>,
    pub sub: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneCards {
    pub public: Vec<SceneCard>,
    pub private: Vec<SceneCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FariScene {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub index_cards: SceneCards,
    pub version: i64,
    pub last_updated: i64,
    pub notes: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board: Option<super::SceneBoard>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabletopState {
    pub scene: Option<FariScene>,
    pub revision: i64,
    pub updated_at: Option<String>,
}

pub(crate) fn validate_scene(scene: &FariScene) -> Result<(), EngineError> {
    validate_name(&scene.name, "Scene name")?;
    if scene.version != 3
        || scene.last_updated < 0
        || scene.notes.chars().count() > 10_000
        || scene
            .group
            .as_ref()
            .is_some_and(|s| s.chars().count() > 120)
    {
        return Err(EngineError::Invalid(
            "Invalid scene version or notes.".into(),
        ));
    }
    let mut ids = HashSet::new();
    let mut add_id = |id: &str| -> Result<(), EngineError> {
        if Uuid::parse_str(id).is_err() || !ids.insert(id.to_string()) {
            return Err(EngineError::Invalid(
                "Scene identifiers must be unique UUIDs.".into(),
            ));
        }
        Ok(())
    };
    add_id(&scene.id)?;
    let cards = scene
        .index_cards
        .public
        .iter()
        .chain(&scene.index_cards.private);
    if scene.index_cards.public.len() + scene.index_cards.private.len() > 50 {
        return Err(EngineError::Invalid(
            "A scene can contain up to 50 cards.".into(),
        ));
    }
    for card in cards {
        add_id(&card.id)?;
        validate_name(&card.title, "Card title")?;
        if card.title_label.trim().is_empty()
            || card.title_label.chars().count() > 40
            || card.color.len() != 7
            || !card.color.starts_with('#')
            || !card.color[1..].bytes().all(|c| c.is_ascii_hexdigit())
            || card.sub
            || !card.sub_cards.is_empty()
            || card.blocks.len() != 1
        {
            return Err(EngineError::Invalid(
                "Only top-level cards with one text block are supported.".into(),
            ));
        }
        let block = &card.blocks[0];
        add_id(&block.id)?;
        if block.block_type != "Text"
            || block.label.chars().count() > 120
            || block.value.chars().count() > 10_000
            || !block.meta.is_empty()
        {
            return Err(EngineError::Invalid("Invalid scene text block.".into()));
        }
    }
    if let Some(board) = &scene.board {
        super::board::validate_board(board, &mut add_id)?;
    }
    if serde_json::to_vec(scene)
        .map_err(|e| EngineError::Invalid(e.to_string()))?
        .len()
        > 1_000_000
    {
        return Err(EngineError::Invalid("Scene data is too large.".into()));
    }
    Ok(())
}

impl CampaignStore {
    pub fn load_tabletop(&self, campaign_id: &str) -> Result<TabletopState, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let row = conn
            .query_row(
                "SELECT scene_json, revision, updated_at FROM tabletops WHERE campaign_id = ?1",
                [campaign_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        match row {
            Some((json, revision, ts)) => {
                let scene: FariScene = serde_json::from_str(&json).map_err(|e| {
                    EngineError::Invalid(format!("Saved scene could not be read: {e}"))
                })?;
                validate_scene(&scene)?;
                Ok(TabletopState {
                    scene: Some(scene),
                    revision,
                    updated_at: Some(ts),
                })
            }
            None => Ok(TabletopState {
                scene: None,
                revision: 0,
                updated_at: None,
            }),
        }
    }

    pub fn save_tabletop(
        &self,
        campaign_id: &str,
        scene: FariScene,
        expected_revision: i64,
    ) -> Result<Saved<TabletopState>, EngineError> {
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        validate_scene(&scene)?;
        if expected_revision < 0 {
            return Err(EngineError::Invalid("Invalid scene revision.".into()));
        }
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        if let Some(board) = &scene.board {
            super::board::validate_board_links(&conn, campaign_id, board)?;
        }
        let timestamp = now();
        let tx = conn.transaction()?;
        let current = tx
            .query_row(
                "SELECT revision FROM tabletops WHERE campaign_id = ?1",
                [campaign_id],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0);
        if current != expected_revision {
            return Err(EngineError::Invalid("This scene changed in another window. Reload it before saving again; your edits are still on screen.".into()));
        }
        let revision = current
            .checked_add(1)
            .ok_or_else(|| EngineError::Invalid("Scene revision limit reached.".into()))?;
        let json =
            serde_json::to_string(&scene).map_err(|e| EngineError::Invalid(e.to_string()))?;
        let count: u32 = tx.query_row(
            "SELECT COUNT(*) FROM tabletop_scenes WHERE campaign_id=?1 AND scene_id != ?2",
            params![campaign_id, scene.id],
            |r| r.get(0),
        )?;
        if count >= 40 {
            return Err(EngineError::Invalid(
                "A campaign supports up to 40 scenes.".into(),
            ));
        }
        tx.execute("INSERT INTO tabletop_scenes (campaign_id,scene_id,scene_json,updated_at) VALUES (?1,?2,?3,?4) ON CONFLICT(campaign_id,scene_id) DO UPDATE SET scene_json=excluded.scene_json,updated_at=excluded.updated_at",params![campaign_id,scene.id,json,timestamp])?;
        if current == 0 {
            tx.execute("INSERT INTO tabletops (campaign_id, scene_json, revision, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![campaign_id, json, revision, timestamp])?;
        } else {
            let changed = tx.execute("UPDATE tabletops SET scene_json = ?1, revision = ?2, updated_at = ?3 WHERE campaign_id = ?4 AND revision = ?5",
                params![json, revision, timestamp, campaign_id, current])?;
            if changed != 1 {
                return Err(EngineError::Invalid(
                    "Scene changed before it could be saved.".into(),
                ));
            }
        }
        record_event(
            &tx,
            campaign_id,
            "scene.saved",
            &serde_json::json!({"scene":scene,"revision":revision}),
            &timestamp,
        )?;
        tx.commit()?;
        let value = TabletopState {
            scene: Some(scene),
            revision,
            updated_at: Some(timestamp),
        };
        Ok(saved(value, &dir, &conn, campaign_id))
    }
}
