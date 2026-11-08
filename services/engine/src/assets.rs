use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TokenSpec {
    pub name: String,
    pub label: String,
    pub color: String,
    pub shape: String,
    pub symbol: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TokenAssetRequest {
    pub spec: TokenSpec,
    pub png_data_url: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedAsset {
    pub id: String,
    pub spec: TokenSpec,
    pub created_at: String,
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn token_svg(spec: &TokenSpec) -> Result<String, EngineError> {
    validate_name(&spec.name, "Asset name")?;
    if spec.label.chars().count() > 18
        || spec.label.chars().any(char::is_control)
        || spec.color.len() != 7
        || !spec.color.starts_with('#')
        || !spec.color[1..].bytes().all(|b| b.is_ascii_hexdigit())
        || !matches!(spec.shape.as_str(), "circle" | "square" | "hex")
        || !matches!(
            spec.symbol.as_str(),
            "shield" | "sword" | "skull" | "star" | "initials"
        )
    {
        return Err(EngineError::Invalid(
            "Choose a token frame, symbol, color, and label up to 18 characters.".into(),
        ));
    }
    let outline = match spec.shape.as_str() {
        "circle" => "<circle cx=\"256\" cy=\"256\" r=\"232\"/>",
        "square" => "<rect x=\"24\" y=\"24\" width=\"464\" height=\"464\" rx=\"42\"/>",
        _ => "<path d=\"M256 18 462 137V375L256 494 50 375V137Z\"/>",
    };
    let symbol=match spec.symbol.as_str(){
        "shield"=>"<path d=\"M256 110 360 150V244Q353 318 256 365 159 318 152 244V150Z\" fill=\"none\" stroke=\"#fff5d6\" stroke-width=\"18\"/><path d=\"M256 145V310M199 218H313\" stroke=\"#fff5d6\" stroke-width=\"16\"/>",
        "sword"=>"<path d=\"M246 104 220 139V289H292V139L266 104Z\" fill=\"#fff5d6\"/><path d=\"M187 292H325M256 299V367M233 374H279\" stroke=\"#fff5d6\" stroke-width=\"19\" stroke-linecap=\"round\"/>",
        "skull"=>"<path d=\"M170 228A86 86 0 1 1 342 228V279L309 294V330H203V294L170 279Z\" fill=\"#fff5d6\"/><circle cx=\"217\" cy=\"230\" r=\"23\" fill=\"#17201b\"/><circle cx=\"295\" cy=\"230\" r=\"23\" fill=\"#17201b\"/><path d=\"M256 261 243 286H269Z\" fill=\"#17201b\"/>",
        "star"=>"<path d=\"M256 107 294 193 388 203 318 267 338 361 256 312 174 361 194 267 124 203 218 193Z\" fill=\"#fff5d6\"/>",
        _=>"",
    };
    let label = xml(&spec.label);
    let (font, y) = if spec.symbol == "initials" {
        (90, 285)
    } else {
        (30, 421)
    };
    Ok(format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512"><title>{}</title><g fill="{}" stroke="#e6d39c" stroke-width="12">{outline}</g>{symbol}<text x="256" y="{y}" fill="#fff5d6" text-anchor="middle" font-family="Georgia,serif" font-size="{font}" font-weight="bold">{label}</text></svg>"##,
        xml(&spec.name),
        spec.color
    ))
}
impl CampaignStore {
    pub fn preview_token_asset(&self, spec: TokenSpec) -> Result<String, EngineError> {
        token_svg(&spec)
    }
    pub fn save_token_asset(
        &self,
        campaign_id: &str,
        request: TokenAssetRequest,
    ) -> Result<Saved<GeneratedAsset>, EngineError> {
        let svg = token_svg(&request.spec)?;
        if request.png_data_url.len() > 5_600_000 {
            return Err(EngineError::Invalid(
                "The PNG token exceeds the size limit.".into(),
            ));
        }
        let data = STANDARD
            .decode(
                request
                    .png_data_url
                    .strip_prefix("data:image/png;base64,")
                    .ok_or_else(|| {
                        EngineError::Invalid("Render the token to PNG before saving.".into())
                    })?,
            )
            .map_err(|_| EngineError::Invalid("Invalid PNG encoding.".into()))?;
        if data.len() < 33
            || data.len() > 4_194_304
            || &data[..8] != b"\x89PNG\r\n\x1a\n"
            || &data[12..16] != b"IHDR"
            || u32::from_be_bytes(data[16..20].try_into().unwrap()) != 512
            || u32::from_be_bytes(data[20..24].try_into().unwrap()) != 512
        {
            return Err(EngineError::Invalid(
                "Save a 512×512 PNG token up to 4 MB.".into(),
            ));
        }
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let tx = conn.transaction()?;
        let (count, total): (i64, i64) = tx.query_row(
            "SELECT COUNT(*),COALESCE(SUM(LENGTH(data)),0) FROM scene_images WHERE campaign_id=?1",
            [campaign_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if count >= 100 || total + data.len() as i64 > 52_428_800 {
            return Err(EngineError::Invalid(
                "This campaign's image allowance is 100 images / 50 MB.".into(),
            ));
        }
        let asset = GeneratedAsset {
            id: Uuid::new_v4().to_string(),
            spec: request.spec,
            created_at: now(),
        };
        tx.execute("INSERT INTO scene_images(id,campaign_id,name,width,height,data,created_at) VALUES (?1,?2,?3,512,512,?4,?5)",params![asset.id,campaign_id,asset.spec.name,data,asset.created_at])?;
        tx.execute("INSERT INTO generated_assets(image_id,campaign_id,spec_json,svg_text,created_at) VALUES (?1,?2,?3,?4,?5)",params![asset.id,campaign_id,serde_json::to_string(&asset.spec).unwrap(),svg,asset.created_at])?;
        record_event(
            &tx,
            campaign_id,
            "asset.created",
            &json!({"assetId":asset.id,"name":asset.spec.name,"formats":["png","svg"]}),
            &asset.created_at,
        )?;
        tx.commit()?;
        Ok(saved(asset, &dir, &conn, campaign_id))
    }
    pub fn list_generated_assets(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<GeneratedAsset>, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let mut stmt=conn.prepare("SELECT image_id,spec_json,created_at FROM generated_assets WHERE campaign_id=?1 ORDER BY created_at DESC")?;
        let rows = stmt.query_map([campaign_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        rows.map(|r| {
            let (id, spec, created_at) = r?;
            Ok(GeneratedAsset {
                id,
                spec: serde_json::from_str(&spec)
                    .map_err(|e| EngineError::Invalid(e.to_string()))?,
                created_at,
            })
        })
        .collect()
    }
    pub fn load_generated_asset(&self, campaign_id: &str, id: &str) -> Result<Value, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let svg: String = conn
            .query_row(
                "SELECT svg_text FROM generated_assets WHERE campaign_id=?1 AND image_id=?2",
                params![campaign_id, id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| {
                EngineError::Invalid("Generated asset not found in this campaign.".into())
            })?;
        Ok(json!({"id":id,"svg":svg,"pngDataUrl":self.load_scene_image(campaign_id,id)?}))
    }
}
