use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
use std::io::Cursor;

pub const CATEGORIES: &[&str] = &[
    "Battle maps",
    "GM references",
    "Scenes",
    "World atlas",
    "Creatures",
    "Table styling",
    "Characters",
    "Items",
    "Token portraits",
    "Unsorted",
];
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtMetadata {
    pub title: String,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub role: String,
    pub note: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtEntry {
    pub id: String,
    pub metadata: ArtMetadata,
    pub width: u32,
    pub height: u32,
    pub has_svg: bool,
    pub generated: bool,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtImport {
    pub metadata: ArtMetadata,
    pub data_url: String,
    pub width: u32,
    pub height: u32,
    pub thumbnail_data_url: String,
    pub source_svg: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtUpdate {
    pub id: String,
    pub metadata: ArtMetadata,
}

pub(crate) fn clean_metadata(mut value: ArtMetadata) -> Result<ArtMetadata, EngineError> {
    value.title = validate_name(&value.title, "Artwork title")?;
    if value.title.chars().any(char::is_control)
        || value.note.chars().count() > 2000
        || value.tags.len() > 30
        || value.categories.len() > CATEGORIES.len()
        || !matches!(value.role.as_str(), "reference" | "background" | "token")
    {
        return Err(EngineError::Invalid(
            "Use a title, up to 30 tags, and notes up to 2000 characters.".into(),
        ));
    }
    if value
        .categories
        .iter()
        .any(|c| !CATEGORIES.contains(&c.as_str()))
    {
        return Err(EngineError::Invalid(
            "Choose artwork categories; use custom tags for additional labels.".into(),
        ));
    }
    value.categories.sort();
    value.categories.dedup();
    if value.categories.is_empty() {
        value.categories.push("Unsorted".into());
    }
    let mut tags = Vec::<String>::new();
    for tag in value.tags {
        let tag = tag.trim().to_string();
        if tag.is_empty() {
            continue;
        }
        if tag.chars().count() > 48 || tag.chars().any(char::is_control) {
            return Err(EngineError::Invalid(
                "Each tag can contain up to 48 characters.".into(),
            ));
        }
        if !tags.iter().any(|t| t.to_lowercase() == tag.to_lowercase()) {
            tags.push(tag);
        }
    }
    value.tags = tags;
    if value.categories.iter().any(|c| c == "GM references") {
        value.role = "reference".into();
    }
    Ok(value)
}

pub(crate) fn decode_png(value: &str, max_edge: u32) -> Result<(Vec<u8>, u32, u32), EngineError> {
    let bad =
        || EngineError::Invalid("Choose a valid PNG up to 4 MB and 4096 pixels per edge.".into());
    if value.len() > 5_600_000 {
        return Err(bad());
    }
    let data = STANDARD
        .decode(
            value
                .strip_prefix("data:image/png;base64,")
                .ok_or_else(bad)?,
        )
        .map_err(|_| bad())?;
    if data.len() > 4_194_304 {
        return Err(bad());
    }
    let decoder = png::Decoder::new(Cursor::new(&data));
    let mut reader = decoder.read_info().map_err(|_| bad())?;
    let (w, h) = (reader.info().width, reader.info().height);
    if w == 0 || h == 0 || w > max_edge || h > max_edge {
        return Err(bad());
    }
    let size = reader.output_buffer_size().ok_or_else(bad)?;
    if size > 134_217_728 {
        return Err(bad());
    }
    reader.next_frame(&mut vec![0; size]).map_err(|_| bad())?;
    reader.finish().map_err(|_|bad())?;
    Ok((data, w, h))
}

// Imported SVGs are a deliberately inert subset. Both rendering and export use this subset.
pub(crate) fn validate_svg(value: &str) -> Result<(), EngineError> {
    use quick_xml::{events::Event, Reader};
    let bad = || {
        EngineError::Invalid("This SVG contains unsupported active or external content. Export it as PNG, or use a self-contained SVG with shapes, text, gradients, and local references.".into())
    };
    if value.len() > 2_000_000 {
        return Err(bad());
    }
    let mut reader = Reader::from_str(value);
    let mut depth = 0usize;
    let mut roots = 0;
    let mut images=0u32;let mut pixels=0u64;
    loop {
        let event = reader.read_event().map_err(|_| bad())?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(ref node) | Event::Empty(ref node) => {
                let name = node.local_name();
                let name = name.as_ref();
                if ![
                    "svg",
                    "g",
                    "defs",
                    "path",
                    "rect",
                    "circle",
                    "ellipse",
                    "line",
                    "polyline",
                    "polygon",
                    "text",
                    "tspan",
                    "title",
                    "desc",
                    "clipPath",
                    "mask",
                    "linearGradient",
                    "radialGradient",
                    "stop",
                    "use",
                    "image",
                ]
                .contains(&name)
                {
                    return Err(bad());
                }
                if depth == 0 {
                    roots += 1;
                    if roots != 1 || name != "svg" {
                        return Err(bad());
                    }
                }
                for attr in node.attributes() {
                    let attr = attr.map_err(|_| bad())?;
                    let key = attr.key.local_name();
                    let key = key.as_ref();
                    let text = attr
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(|_| bad())?;
                    let lower = text.to_lowercase();
                    if key=="href" && name=="image" && text.starts_with("data:image/png;base64,") {
                        let(_,w,h)=decode_png(&text,4096)?;images+=1;pixels+=w as u64*h as u64;if images>16||pixels>16_777_216{return Err(bad());}
                    }else if key=="href"&&!text.starts_with('#'){return Err(bad());}
                    if key.to_lowercase().starts_with("on")
                        || key == "style"
                        || !svg_attribute(key)
                        || lower.contains("\\")
                        || lower.contains('@')
                        || lower.contains("javascript:")
                        || (lower.contains("url(") && !lower.starts_with("url(#"))
                    {
                        return Err(bad());
                    }
                    if lower.starts_with("url(#")
                        && (!lower.ends_with(')')
                            || lower[5..lower.len() - 1].contains(['(', ')', '\'', '"']))
                    {
                        return Err(bad());
                    }
                }
                if !empty {
                    depth += 1;
                }
                if depth > 64 {
                    return Err(bad());
                }
            }
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or_else(bad)?;
            }
            Event::DocType(_) | Event::PI(_) => return Err(bad()),
            Event::Eof => break,
            Event::Text(ref t) if depth == 0 && !t.as_ref().trim().is_empty() => return Err(bad()),
            _ => {}
        }
    }
    if roots != 1 || depth != 0 {
        return Err(bad());
    }
    Ok(())
}
fn svg_attribute(key: &str) -> bool {
    [
        "xmlns",
        "xlink",
        "version",
        "width",
        "height",
        "viewBox",
        "preserveAspectRatio",
        "id",
        "d",
        "x",
        "y",
        "x1",
        "y1",
        "x2",
        "y2",
        "dx",
        "dy",
        "cx",
        "cy",
        "r",
        "rx",
        "ry",
        "points",
        "transform",
        "fill",
        "fill-rule",
        "fill-opacity",
        "stroke",
        "stroke-width",
        "stroke-opacity",
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-dasharray",
        "stroke-dashoffset",
        "opacity",
        "clip-path",
        "clip-rule",
        "mask",
        "font-family",
        "font-size",
        "font-weight",
        "font-style",
        "text-anchor",
        "dominant-baseline",
        "gradientUnits",
        "gradientTransform",
        "offset",
        "stop-color",
        "stop-opacity",
        "fx",
        "fy",
        "fr",
        "href",
        "color",
        "paint-order",
    ]
    .contains(&key)
}
pub(crate) fn image_allowance(
    tx: &Transaction<'_>,
    campaign_id: &str,
    bytes: usize,
) -> Result<(), EngineError> {
    let (count, total): (i64, i64) = tx.query_row(
        "SELECT COUNT(*),COALESCE(SUM(LENGTH(data)),0) FROM scene_images WHERE campaign_id=?1",
        [campaign_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if count >= 500 || total + bytes as i64 > 536_870_912 {
        return Err(EngineError::Invalid(
            "This campaign's image allowance is 500 images / 512 MB.".into(),
        ));
    }
    Ok(())
}
impl CampaignStore {
    pub fn import_art(
        &self,
        campaign_id: &str,
        request: ArtImport,
    ) -> Result<Saved<ArtEntry>, EngineError> {
        let metadata = clean_metadata(request.metadata)?;
        let (data, w, h) = decode_png(&request.data_url, 4096)?;
        if (w, h) != (request.width, request.height) {
            return Err(EngineError::Invalid(
                "Image dimensions do not match.".into(),
            ));
        }
        let (thumbnail, _, _) = decode_png(&request.thumbnail_data_url, 256)?;
        if let Some(ref svg) = request.source_svg {
            validate_svg(svg)?;
        }
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let tx = conn.transaction()?;
        image_allowance(&tx, campaign_id, data.len())?;
        let id = Uuid::new_v4().to_string();
        let time = now();
        tx.execute("INSERT INTO scene_images(id,campaign_id,name,width,height,data,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![id,campaign_id,metadata.title,w,h,data,time])?;
        tx.execute("INSERT INTO art_metadata(image_id,campaign_id,metadata_json,source_svg,thumbnail,updated_at) VALUES(?1,?2,?3,?4,?5,?6)",params![id,campaign_id,serde_json::to_string(&metadata).unwrap(),request.source_svg,thumbnail,time])?;
        record_event(
            &tx,
            campaign_id,
            "art.imported",
            &json!({"imageId":id,"name":metadata.title,"categories":metadata.categories,"tags":metadata.tags}),
            &time,
        )?;
        tx.commit()?;
        Ok(saved(
            ArtEntry {
                id,
                metadata,
                width: w,
                height: h,
                has_svg: request.source_svg.is_some(),
                generated: false,
            },
            &dir,
            &conn,
            campaign_id,
        ))
    }
    pub fn list_art(&self, campaign_id: &str) -> Result<Vec<ArtEntry>, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let mut stmt=conn.prepare("SELECT i.id,i.name,i.width,i.height,a.metadata_json,a.source_svg IS NOT NULL,g.image_id IS NOT NULL FROM scene_images i LEFT JOIN art_metadata a ON a.image_id=i.id LEFT JOIN generated_assets g ON g.image_id=i.id WHERE i.campaign_id=?1 ORDER BY i.created_at DESC,i.id")?;
        let rows = stmt.query_map([campaign_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, bool>(5)?,
                r.get::<_, bool>(6)?,
            ))
        })?;
        rows.map(|row| {
            let (id, title, width, height, metadata, svg, generated) = row?;
            let metadata = match metadata {
                Some(m) => {
                    serde_json::from_str(&m).map_err(|e| EngineError::Invalid(e.to_string()))?
                }
                None => ArtMetadata {
                    title,
                    categories: vec![if generated {
                        "Token portraits"
                    } else {
                        "Unsorted"
                    }
                    .into()],
                    tags: vec![],
                    role: if generated { "token" } else { "reference" }.into(),
                    note: String::new(),
                },
            };
            Ok(ArtEntry {
                id,
                metadata,
                width,
                height,
                has_svg: svg || generated,
                generated,
            })
        })
        .collect()
    }
    pub fn update_art(
        &self,
        campaign_id: &str,
        request: ArtUpdate,
    ) -> Result<Saved<ArtMetadata>, EngineError> {
        let metadata = clean_metadata(request.metadata)?;
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let tx = conn.transaction()?;
        if tx.execute(
            "UPDATE scene_images SET name=?3 WHERE campaign_id=?1 AND id=?2",
            params![campaign_id, request.id, metadata.title],
        )? != 1
        {
            return Err(EngineError::Invalid(
                "Artwork not found in this campaign.".into(),
            ));
        }
        let time = now();
        tx.execute("INSERT INTO art_metadata(image_id,campaign_id,metadata_json,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(image_id) DO UPDATE SET metadata_json=excluded.metadata_json,updated_at=excluded.updated_at",params![request.id,campaign_id,serde_json::to_string(&metadata).unwrap(),time])?;
        record_event(
            &tx,
            campaign_id,
            "art.tagged",
            &json!({"imageId":request.id,"name":metadata.title,"tags":metadata.tags,"categories":metadata.categories}),
            &time,
        )?;
        tx.commit()?;
        Ok(saved(metadata, &dir, &conn, campaign_id))
    }
    pub fn load_art_preview(&self, campaign_id: &str, id: &str) -> Result<String, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let thumbnail: Option<Vec<u8>> = conn
            .query_row(
                "SELECT thumbnail FROM art_metadata WHERE campaign_id=?1 AND image_id=?2",
                params![campaign_id, id],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        match thumbnail {
            Some(data) => Ok(format!("data:image/png;base64,{}", STANDARD.encode(data))),
            None => self.load_scene_image(campaign_id, id),
        }
    }
    pub fn load_art_svg(&self, campaign_id: &str, id: &str) -> Result<String, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        conn.query_row("SELECT source_svg FROM art_metadata WHERE campaign_id=?1 AND image_id=?2 AND source_svg IS NOT NULL UNION ALL SELECT svg_text FROM generated_assets WHERE campaign_id=?1 AND image_id=?2 LIMIT 1",params![campaign_id,id],|r|r.get(0)).optional()?.ok_or_else(||EngineError::Invalid("No SVG is saved for this artwork.".into()))
    }
}
