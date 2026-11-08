use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BoardPoint {
    pub x: f64,
    pub y: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenKind {
    Player,
    Npc,
    Monster,
    Object,
    Treasure,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BoardToken {
    pub id: String,
    pub name: String,
    pub kind: TokenKind,
    pub x: f64,
    pub y: f64,
    pub size: u32,
    pub color: String,
    pub sprite: Option<String>,
    pub character_id: Option<String>,
    pub notes: String,
    pub states: Vec<String>,
    pub hp: Option<i64>,
    pub max_hp: Option<i64>,
    pub hidden: bool,
    pub locked: bool,
    pub rotation: f64,
    pub bright_light: f64,
    pub dim_light: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DrawingKind {
    Pen,
    Rectangle,
    Ellipse,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BoardDrawing {
    pub id: String,
    pub kind: DrawingKind,
    pub points: Vec<BoardPoint>,
    pub color: String,
    pub width: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FogMark {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub reveal: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneBoard {
    pub columns: u32,
    pub rows: u32,
    pub feet_per_cell: u32,
    pub grid: bool,
    pub snap: bool,
    pub grid_opacity: f64,
    pub background: Option<String>,
    #[serde(default = "default_background_fit")]
    pub background_fit: String,
    #[serde(default = "default_background_scale")]
    pub background_scale: f64,
    #[serde(default)]
    pub background_x: f64,
    #[serde(default)]
    pub background_y: f64,
    pub tokens: Vec<BoardToken>,
    pub drawings: Vec<BoardDrawing>,
    pub fog_enabled: bool,
    pub fog: Vec<FogMark>,
    pub ambient: f64,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneSummary {
    pub id: String,
    pub name: String,
    pub background: Option<String>,
    pub updated_at: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneImage {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageImport {
    pub name: String,
    pub data_url: String,
    pub width: u32,
    pub height: u32,
}

pub(crate) const BUILTIN_IMAGES: &[&str] = &[
    "builtin:pond",
    "builtin:forest",
    "builtin:sewer",
    "builtin:orrek",
];
fn default_background_fit() -> String {
    "contain".into()
}
fn default_background_scale() -> f64 {
    1.0
}
fn is_builtin_image(id: &str) -> bool {
    static ART_IDS: std::sync::OnceLock<HashSet<String>> = std::sync::OnceLock::new();
    BUILTIN_IMAGES.contains(&id)
        || ART_IDS
            .get_or_init(|| {
                let value: Value = serde_json::from_str(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../packages/shared/assets/catalog.json"
                )))
                .expect("bundled artwork catalog");
                value["assets"]
                    .as_array()
                    .expect("artwork entries")
                    .iter()
                    .filter_map(|a| a["id"].as_str().map(str::to_string))
                    .collect()
            })
            .contains(id)
}
fn finite_between(v: f64, min: f64, max: f64) -> bool {
    v.is_finite() && (min..=max).contains(&v)
}
fn color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|c| c.is_ascii_hexdigit())
}
fn valid_image_ref(s: &str) -> bool {
    is_builtin_image(s) || Uuid::parse_str(s).is_ok()
}
pub(crate) fn validate_board(
    board: &SceneBoard,
    add_id: &mut impl FnMut(&str) -> Result<(), EngineError>,
) -> Result<(), EngineError> {
    let invalid =
        || EngineError::Invalid("Invalid tabletop board, token, drawing, or fog settings.".into());
    if !(8..=100).contains(&board.columns)
        || !(8..=100).contains(&board.rows)
        || board.feet_per_cell != 5
        || !finite_between(board.grid_opacity, 0.0, 0.8)
        || !finite_between(board.ambient, 0.0, 1.0)
        || !matches!(board.background_fit.as_str(), "contain" | "stretch")
        || !finite_between(board.background_scale, 0.25, 4.0)
        || !finite_between(board.background_x, -100.0, 100.0)
        || !finite_between(board.background_y, -100.0, 100.0)
        || board.tokens.len() > 100
        || board.drawings.len() > 200
        || board.fog.len() > 500
        || board
            .background
            .as_ref()
            .is_some_and(|s| !valid_image_ref(s))
    {
        return Err(invalid());
    }
    let point = |x, y| {
        finite_between(x, 0.0, f64::from(board.columns))
            && finite_between(y, 0.0, f64::from(board.rows))
    };
    for token in &board.tokens {
        add_id(&token.id)?;
        validate_name(&token.name, "Token name")?;
        if !(1..=8).contains(&token.size)
            || !point(token.x, token.y)
            || token.x + f64::from(token.size) > f64::from(board.columns)
            || token.y + f64::from(token.size) > f64::from(board.rows)
            || !color(&token.color)
            || !finite_between(token.rotation, 0.0, 360.0)
            || !finite_between(token.bright_light, 0.0, 300.0)
            || !finite_between(token.dim_light, token.bright_light, 300.0)
            || token.notes.chars().count() > 10000
            || token.states.len() > 16
            || token
                .states
                .iter()
                .any(|s| s.trim().is_empty() || s.chars().count() > 80)
            || token.sprite.as_ref().is_some_and(|s| !valid_image_ref(s))
            || token
                .character_id
                .as_ref()
                .is_some_and(|s| Uuid::parse_str(s).is_err())
            || token.hp.is_some_and(|v| !(0..=9999).contains(&v))
            || token.max_hp.is_some_and(|v| !(1..=9999).contains(&v))
            || token.hp.zip(token.max_hp).is_some_and(|(hp, max)| hp > max)
        {
            return Err(invalid());
        }
    }
    for drawing in &board.drawings {
        add_id(&drawing.id)?;
        if drawing.points.len() < 2
            || drawing.points.len() > 2000
            || !finite_between(drawing.width, 0.02, 0.5)
            || !color(&drawing.color)
            || drawing.points.iter().any(|p| !point(p.x, p.y))
            || (!matches!(drawing.kind, DrawingKind::Pen) && drawing.points.len() != 2)
        {
            return Err(invalid());
        }
    }
    for mark in &board.fog {
        add_id(&mark.id)?;
        if !point(mark.x, mark.y) || !finite_between(mark.radius, 0.5, 20.0) {
            return Err(invalid());
        }
    }
    Ok(())
}
pub(crate) fn validate_board_links(
    conn: &Connection,
    campaign_id: &str,
    board: &SceneBoard,
) -> Result<(), EngineError> {
    let refs: HashSet<&str> = board
        .background
        .iter()
        .map(String::as_str)
        .chain(board.tokens.iter().filter_map(|t| t.sprite.as_deref()))
        .collect();
    for id in refs {
        if is_builtin_image(id) {
            continue;
        }
        let found: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM scene_images WHERE campaign_id=?1 AND id=?2)",
            params![campaign_id, id],
            |r| r.get(0),
        )?;
        if !found {
            return Err(EngineError::Invalid(
                "An image is missing from this campaign. Import it before saving.".into(),
            ));
        }
    }
    for id in board
        .tokens
        .iter()
        .filter_map(|t| t.character_id.as_deref())
    {
        let found: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM characters WHERE campaign_id=?1 AND id=?2)",
            params![campaign_id, id],
            |r| r.get(0),
        )?;
        if !found {
            return Err(EngineError::CharacterNotFound(id.to_string()));
        }
    }
    Ok(())
}
impl CampaignStore {
    pub fn list_tabletop_scenes(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<SceneSummary>, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let mut stmt=conn.prepare("SELECT scene_json, updated_at FROM tabletop_scenes WHERE campaign_id=?1 ORDER BY updated_at DESC, scene_id")?;
        let rows = stmt.query_map([campaign_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        rows.map(|row| {
            let (json, ts) = row?;
            let scene: FariScene =
                serde_json::from_str(&json).map_err(|e| EngineError::Invalid(e.to_string()))?;
            Ok(SceneSummary {
                id: scene.id,
                name: scene.name,
                background: scene.board.and_then(|b| b.background),
                updated_at: ts,
            })
        })
        .collect()
    }
    pub fn select_tabletop_scene(
        &self,
        campaign_id: &str,
        scene_id: &str,
        expected_revision: i64,
    ) -> Result<Saved<TabletopState>, EngineError> {
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let tx = conn.transaction()?;
        let json: String = tx
            .query_row(
                "SELECT scene_json FROM tabletop_scenes WHERE campaign_id=?1 AND scene_id=?2",
                params![campaign_id, scene_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| EngineError::Invalid("Scene not found in this campaign.".into()))?;
        let scene: FariScene =
            serde_json::from_str(&json).map_err(|e| EngineError::Invalid(e.to_string()))?;
        super::tabletop::validate_scene(&scene)?;
        let current: i64 = tx
            .query_row(
                "SELECT revision FROM tabletops WHERE campaign_id=?1",
                [campaign_id],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0);
        if expected_revision < 0 || current != expected_revision {
            return Err(EngineError::Invalid(
                "This scene changed in another window. Refresh before selecting a scene.".into(),
            ));
        }
        let revision = current
            .checked_add(1)
            .ok_or_else(|| EngineError::Invalid("Scene revision limit reached.".into()))?;
        let ts = now();
        tx.execute("INSERT INTO tabletops (campaign_id,scene_json,revision,updated_at) VALUES (?1,?2,?3,?4) ON CONFLICT(campaign_id) DO UPDATE SET scene_json=excluded.scene_json,revision=excluded.revision,updated_at=excluded.updated_at",params![campaign_id,json,revision,ts])?;
        record_event(
            &tx,
            campaign_id,
            "scene.selected",
            &serde_json::json!({"sceneId":scene_id,"name":scene.name,"revision":revision}),
            &ts,
        )?;
        tx.commit()?;
        Ok(saved(
            TabletopState {
                scene: Some(scene),
                revision,
                updated_at: Some(ts),
            },
            &dir,
            &conn,
            campaign_id,
        ))
    }
    pub fn list_scene_images(&self, campaign_id: &str) -> Result<Vec<SceneImage>, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let mut stmt=conn.prepare("SELECT id,name,width,height FROM scene_images WHERE campaign_id=?1 ORDER BY created_at DESC")?;
        let rows = stmt.query_map([campaign_id], |r| {
            Ok(SceneImage {
                id: r.get(0)?,
                name: r.get(1)?,
                width: r.get(2)?,
                height: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    pub fn load_scene_image(
        &self,
        campaign_id: &str,
        image_id: &str,
    ) -> Result<String, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let data: Vec<u8> = conn
            .query_row(
                "SELECT data FROM scene_images WHERE campaign_id=?1 AND id=?2",
                params![campaign_id, image_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| EngineError::Invalid("Image not found in this campaign.".into()))?;
        Ok(format!("data:image/png;base64,{}", STANDARD.encode(data)))
    }
    pub fn import_scene_image(
        &self,
        campaign_id: &str,
        request: ImageImport,
    ) -> Result<Saved<SceneImage>, EngineError> {
        validate_name(&request.name, "Image name")?;
        if request.data_url.len() > 5_600_000
            || request.width == 0
            || request.height == 0
            || request.width > 4096
            || request.height > 4096
        {
            return Err(EngineError::Invalid(
                "Use an image up to 4096 pixels per edge and 4 MB after conversion.".into(),
            ));
        }
        let encoded = request
            .data_url
            .strip_prefix("data:image/png;base64,")
            .ok_or_else(|| EngineError::Invalid("Import images as PNG.".into()))?;
        let data = STANDARD
            .decode(encoded)
            .map_err(|_| EngineError::Invalid("Invalid image data.".into()))?;
        if data.len() < 33
            || data.len() > 4_194_304
            || &data[..8] != b"\x89PNG\r\n\x1a\n"
            || &data[12..16] != b"IHDR"
            || u32::from_be_bytes(data[16..20].try_into().unwrap()) != request.width
            || u32::from_be_bytes(data[20..24].try_into().unwrap()) != request.height
        {
            return Err(EngineError::Invalid(
                "Image dimensions or PNG header are invalid.".into(),
            ));
        }
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let tx = conn.transaction()?;
        let (count, total): (u32, u64) = tx.query_row(
            "SELECT COUNT(*),COALESCE(SUM(LENGTH(data)),0) FROM scene_images WHERE campaign_id=?1",
            [campaign_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if count >= 500 || total + data.len() as u64 > 536_870_912 {
            return Err(EngineError::Invalid(
                "This campaign's image allowance is 500 images / 512 MB.".into(),
            ));
        }
        let image = SceneImage {
            id: Uuid::new_v4().to_string(),
            name: request.name.trim().to_string(),
            width: request.width,
            height: request.height,
        };
        let ts = now();
        tx.execute("INSERT INTO scene_images(id,campaign_id,name,width,height,data,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",params![image.id,campaign_id,image.name,image.width,image.height,data,ts])?;
        record_event(
            &tx,
            campaign_id,
            "image.imported",
            &serde_json::to_value(&image).unwrap(),
            &ts,
        )?;
        tx.commit()?;
        Ok(saved(image, &dir, &conn, campaign_id))
    }
}
