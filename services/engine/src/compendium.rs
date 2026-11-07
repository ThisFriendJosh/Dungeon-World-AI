use super::*;
fn text_or_empty<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentEntry {
    pub name: String,
    #[serde(default, deserialize_with = "text_or_empty")]
    pub description: String,
    #[serde(default)]
    pub properties: Value,
    #[serde(default, deserialize_with = "text_or_empty")]
    pub publisher: String,
    #[serde(default, deserialize_with = "text_or_empty")]
    pub book: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentImport {
    pub name: String,
    pub kind: String,
    pub entries: Vec<ContentEntry>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentPack {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub entry_count: i64,
    pub imported_at: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentQuery {
    pub search: String,
    pub kind: String,
    pub edition: String,
    pub book: String,
    pub offset: u32,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSummary {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub book: String,
    pub publisher: String,
    pub source_edition: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentResults {
    pub total: i64,
    pub entries: Vec<ContentSummary>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentDetail {
    pub entry: ContentEntry,
    pub source_edition: String,
    pub kind: String,
}
fn kind_valid(kind: &str) -> bool {
    [
        "backgrounds",
        "classes",
        "items",
        "monsters",
        "species",
        "spells",
    ]
    .contains(&kind)
}
fn source_edition(book: &str) -> &'static str {
    // Only explicit year labels classify a source. An unmarked PHB/MM title is ambiguous.
    if book.contains("2024") || book.contains("2025") {
        "5e-2024"
    } else if book.contains("2014") {
        "5e-2014"
    } else {
        "unassigned"
    }
}
impl CampaignStore {
    pub fn import_content(
        &self,
        campaign_id: &str,
        request: ContentImport,
    ) -> Result<Saved<ContentPack>, EngineError> {
        let name = validate_name(&request.name, "Collection name")?;
        if !kind_valid(&request.kind) || request.entries.is_empty() || request.entries.len() > 20000
        {
            return Err(EngineError::Invalid(
                "Import 1-20000 entries from one supported JSON dataset.".into(),
            ));
        }
        let mut encoded = Vec::with_capacity(request.entries.len());
        let mut bytes = 0usize;
        for entry in &request.entries {
            validate_name(&entry.name, "Entry name")?;
            if entry.description.len() > 1_000_000
                || entry.book.len() > 500
                || entry.publisher.len() > 500
                || (!entry.properties.is_object() && !entry.properties.is_null())
            {
                return Err(EngineError::Invalid(
                    "The collection contains an unsupported entry.".into(),
                ));
            }
            let json =
                serde_json::to_string(entry).map_err(|e| EngineError::Invalid(e.to_string()))?;
            bytes = bytes
                .checked_add(json.len())
                .ok_or_else(|| EngineError::Invalid("Collection is too large.".into()))?;
            if bytes > 40_000_000 {
                return Err(EngineError::Invalid(
                    "One collection may contain up to 40 MB of data.".into(),
                ));
            }
            encoded.push(json);
        }
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let tx = conn.transaction()?;
        let (packs,total,stored):(i64,i64,i64)=tx.query_row("SELECT COUNT(*),COALESCE(SUM(entry_count),0),COALESCE(SUM(byte_count),0) FROM compendium_packs WHERE campaign_id=?1",[campaign_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        if packs >= 16
            || total + request.entries.len() as i64 > 50000
            || stored + bytes as i64 > 100_000_000
        {
            return Err(EngineError::Invalid("This campaign supports 16 collections, 50000 entries, and 100 MB of reference data.".into()));
        }
        let id = Uuid::new_v4().to_string();
        let timestamp = now();
        tx.execute("INSERT INTO compendium_packs(id,campaign_id,name,kind,entry_count,byte_count,imported_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",params![id,campaign_id,name,request.kind,request.entries.len() as i64,bytes as i64,timestamp])?;
        {
            let mut insert=tx.prepare("INSERT INTO compendium_entries(id,pack_id,name,book,publisher,source_edition,content_json) VALUES (?1,?2,?3,?4,?5,?6,?7)")?;
            for (entry, json) in request.entries.iter().zip(encoded.iter()) {
                insert.execute(params![
                    Uuid::new_v4().to_string(),
                    id,
                    entry.name.trim(),
                    entry.book,
                    entry.publisher,
                    source_edition(&entry.book),
                    json
                ])?;
            }
        }
        let pack = ContentPack {
            id,
            name,
            kind: request.kind,
            entry_count: request.entries.len() as i64,
            imported_at: timestamp.clone(),
        };
        record_event(
            &tx,
            campaign_id,
            "content.imported",
            &serde_json::to_value(&pack).map_err(|e| EngineError::Invalid(e.to_string()))?,
            &timestamp,
        )?;
        tx.commit()?;
        Ok(saved(pack, &dir, &conn, campaign_id))
    }
    pub fn list_content_packs(&self, campaign_id: &str) -> Result<Vec<ContentPack>, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let mut stmt=conn.prepare("SELECT id,name,kind,entry_count,imported_at FROM compendium_packs WHERE campaign_id=?1 ORDER BY imported_at,id")?;
        let rows = stmt.query_map([campaign_id], |r| {
            Ok(ContentPack {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: r.get(2)?,
                entry_count: r.get(3)?,
                imported_at: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    pub fn list_content_books(&self, campaign_id: &str) -> Result<Vec<String>, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let mut stmt=conn.prepare("SELECT DISTINCT e.book FROM compendium_entries e JOIN compendium_packs p ON p.id=e.pack_id WHERE p.campaign_id=?1 ORDER BY e.book COLLATE NOCASE")?;
        let rows = stmt.query_map([campaign_id], |r| r.get(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    pub fn search_content(
        &self,
        campaign_id: &str,
        query: ContentQuery,
    ) -> Result<ContentResults, EngineError> {
        if query.search.chars().count() > 120
            || (!query.kind.is_empty() && !kind_valid(&query.kind))
            || !["", "5e-2014", "5e-2024", "unassigned"].contains(&query.edition.as_str())
            || query.book.len() > 500
            || query.offset > 50000
        {
            return Err(EngineError::Invalid(
                "Unsupported compendium search.".into(),
            ));
        }
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let needle = format!(
            "%{}%",
            query
                .search
                .trim()
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        let filter=" FROM compendium_entries e JOIN compendium_packs p ON p.id=e.pack_id WHERE p.campaign_id=?1 AND (e.name LIKE ?2 ESCAPE '\\' OR e.publisher LIKE ?2 ESCAPE '\\') AND (?3='' OR p.kind=?3) AND (?4='' OR e.source_edition=?4) AND (?5='' OR e.book=?5)";
        let total = conn.query_row(
            &format!("SELECT COUNT(*){filter}"),
            params![campaign_id, needle, query.kind, query.edition, query.book],
            |r| r.get(0),
        )?;
        let mut stmt=conn.prepare(&format!("SELECT e.id,e.name,p.kind,e.book,e.publisher,e.source_edition{filter} ORDER BY e.name COLLATE NOCASE,e.book,e.id LIMIT 30 OFFSET ?6"))?;
        let rows = stmt.query_map(
            params![
                campaign_id,
                needle,
                query.kind,
                query.edition,
                query.book,
                query.offset
            ],
            |r| {
                Ok(ContentSummary {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    kind: r.get(2)?,
                    book: r.get(3)?,
                    publisher: r.get(4)?,
                    source_edition: r.get(5)?,
                })
            },
        )?;
        Ok(ContentResults {
            total,
            entries: rows.collect::<Result<Vec<_>, _>>()?,
        })
    }
    pub fn load_content_entry(
        &self,
        campaign_id: &str,
        entry_id: &str,
    ) -> Result<ContentDetail, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let (json,source_edition,kind):(String,String,String)=conn.query_row("SELECT e.content_json,e.source_edition,p.kind FROM compendium_entries e JOIN compendium_packs p ON p.id=e.pack_id WHERE p.campaign_id=?1 AND e.id=?2",params![campaign_id,entry_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        Ok(ContentDetail {
            entry: serde_json::from_str(&json).map_err(|e| EngineError::Invalid(e.to_string()))?,
            source_edition,
            kind,
        })
    }
}
