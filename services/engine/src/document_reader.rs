use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OriginalDocumentImport {
    pub name: String,
    pub data_base64: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalDocument {
    pub name: String,
    pub media_type: String,
    pub byte_count: i64,
    pub saved_at: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPassage {
    pub id: String,
    pub ordinal: u32,
    pub text: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeDocument {
    pub source: KnowledgeSource,
    pub original: Option<OriginalDocument>,
    pub passages: Vec<DocumentPassage>,
    pub offset: u32,
    pub next_offset: Option<u32>,
}

pub(crate) fn decode_original(
    value: OriginalDocumentImport,
) -> Result<(String, String, Vec<u8>), EngineError> {
    if value.name.is_empty()
        || value.name.len() > 255
        || value.name.contains(['/', '\\', ':', '\0'])
        || value.data_base64.len() > 100_000_000
    {
        return Err(EngineError::Invalid(
            "Choose a local document up to 75 MB with a simple filename.".into(),
        ));
    }
    let extension = value
        .name
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_lowercase();
    let mime = match extension.as_str() {
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "txt" | "md" | "markdown" | "csv" => "text/plain",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        _ => {
            return Err(EngineError::Invalid(
                "Choose PDF, DOCX, TXT, Markdown, CSV, HTML or JSON.".into(),
            ))
        }
    };
    let bytes = STANDARD
        .decode(value.data_base64)
        .map_err(|_| EngineError::Invalid("The original document could not be read.".into()))?;
    if bytes.is_empty()
        || bytes.len() > 75_000_000
        || (extension == "pdf" && !bytes.starts_with(b"%PDF-"))
        || (extension == "docx" && !bytes.starts_with(b"PK\x03\x04"))
    {
        return Err(EngineError::Invalid(
            "The original document is empty, damaged, or over 75 MB.".into(),
        ));
    }
    Ok((value.name, mime.into(), bytes))
}
pub(crate) fn original_metadata(
    conn: &Connection,
    id: &str,
) -> Result<Option<OriginalDocument>, EngineError> {
    Ok(conn.query_row("SELECT name,media_type,byte_count,saved_at FROM knowledge_originals WHERE source_id=?1",[id],|r|Ok(OriginalDocument{name:r.get(0)?,media_type:r.get(1)?,byte_count:r.get(2)?,saved_at:r.get(3)?})).optional()?)
}
pub(crate) fn save_original(
    tx: &Transaction<'_>,
    id: &str,
    value: &(String, String, Vec<u8>),
) -> Result<(), EngineError> {
    tx.execute("INSERT INTO knowledge_originals(source_id,name,media_type,bytes,byte_count,saved_at) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(source_id) DO UPDATE SET name=excluded.name,media_type=excluded.media_type,bytes=excluded.bytes,byte_count=excluded.byte_count,saved_at=excluded.saved_at",params![id,value.0,value.1,value.2,value.2.len() as i64,now()])?;
    Ok(())
}
impl CampaignStore {
    pub fn read_knowledge_document(
        &self,
        campaign_id: &str,
        id: &str,
        offset: u32,
    ) -> Result<KnowledgeDocument, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let source_id:Option<String>=conn.query_row("SELECT s.id FROM knowledge_sources s WHERE s.campaign_id=?1 AND (s.id=?2 OR EXISTS(SELECT 1 FROM knowledge_chunks c WHERE c.source_id=s.id AND c.id=?2))",params![campaign_id,id],|r|r.get(0)).optional()?;
        let source = self
            .list_knowledge_sources(campaign_id)?
            .into_iter()
            .find(|s| Some(&s.id) == source_id.as_ref())
            .ok_or_else(|| EngineError::Invalid("Document not found in this campaign.".into()))?;
        let id = source.id.as_str();
        if offset >= source.chunk_count.max(1) as u32 {
            return Err(EngineError::Invalid(
                "Choose a section in this document.".into(),
            ));
        }
        let mut stmt=conn.prepare("SELECT id,ordinal,body FROM knowledge_chunks WHERE source_id=?1 ORDER BY ordinal LIMIT 6 OFFSET ?2")?;
        let passages = stmt
            .query_map(params![id, offset], |r| {
                Ok(DocumentPassage {
                    id: r.get(0)?,
                    ordinal: r.get(1)?,
                    text: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let next_offset = (offset + 6 < source.chunk_count as u32).then_some(offset + 6);
        Ok(KnowledgeDocument {
            original: original_metadata(&conn, id)?,
            source,
            passages,
            offset,
            next_offset,
        })
    }
    pub fn search_knowledge_document(
        &self,
        campaign_id: &str,
        id: &str,
        query: &str,
    ) -> Result<Vec<DocumentPassage>, EngineError> {
        let document = self.read_knowledge_document(campaign_id, id, 0)?;
        let id = document.source.id.as_str();
        if query.trim().is_empty() || query.chars().count() > 200 {
            return Err(EngineError::Invalid(
                "Search this document with 1–200 characters.".into(),
            ));
        }
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let mut stmt=conn.prepare("SELECT id,ordinal,body FROM knowledge_chunks WHERE source_id=?1 AND instr(lower(body),lower(?2))>0 ORDER BY ordinal LIMIT 100")?;
        let rows = stmt
            .query_map(params![id, query.trim()], |r| {
                Ok(DocumentPassage {
                    id: r.get(0)?,
                    ordinal: r.get(1)?,
                    text: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
    pub fn attach_knowledge_original(
        &self,
        campaign_id: &str,
        id: &str,
        original: OriginalDocumentImport,
    ) -> Result<Saved<OriginalDocument>, EngineError> {
        let original = decode_original(original)?;
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM knowledge_sources WHERE campaign_id=?1 AND id=?2)",
            params![campaign_id, id],
            |r| r.get(0),
        )?;
        if !exists {
            return Err(EngineError::Invalid(
                "Document not found in this campaign.".into(),
            ));
        }
        let tx = conn.transaction()?;
        save_original(&tx, id, &original)?;
        record_event(
            &tx,
            campaign_id,
            "knowledge.original_saved",
            &serde_json::json!({"sourceId":id,"name":original.0}),
            &now(),
        )?;
        tx.commit()?;
        Ok(saved(
            original_metadata(&conn, id)?.unwrap(),
            &dir,
            &conn,
            campaign_id,
        ))
    }
    pub fn knowledge_original_bytes(
        &self,
        campaign_id: &str,
        id: &str,
    ) -> Result<(OriginalDocument, Vec<u8>), EngineError> {
        let document = self.read_knowledge_document(campaign_id, id, 0)?;
        let id = document.source.id.as_str();
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let metadata=original_metadata(&conn,id)?.ok_or_else(||EngineError::Invalid("This older import saved text only. Attach its original file to view the original layout.".into()))?;
        let bytes = conn.query_row(
            "SELECT bytes FROM knowledge_originals WHERE source_id=?1",
            [id],
            |r| r.get(0),
        )?;
        Ok((metadata, bytes))
    }
}
