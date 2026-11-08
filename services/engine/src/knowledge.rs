use super::*;
use serde_json::json;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeImport {
    pub title: String,
    pub text: String,
    #[serde(default)]
    pub source_ref: String,
    #[serde(default)]
    pub edition: String,
    #[serde(default = "gm_default")]
    pub gm_only: bool,
}
fn gm_default() -> bool {
    true
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSource {
    pub id: String,
    pub title: String,
    pub source_ref: String,
    pub edition: String,
    pub gm_only: bool,
    pub chunk_count: i64,
    pub imported_at: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceHit {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub book: String,
    pub edition: String,
    pub gm_only: bool,
    pub source_ref: String,
    pub excerpt: String,
}
pub(crate) fn clip(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}
fn chunks(text: &str) -> Vec<String> {
    let characters: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut position = 0;
    while position < characters.len() {
        let mut end = (position + 1800).min(characters.len());
        if end < characters.len() {
            if let Some(offset) = characters[position + 900..end]
                .iter()
                .rposition(|c| c.is_whitespace())
            {
                end = position + 900 + offset + 1;
            }
        }
        let part: String = characters[position..end].iter().collect();
        if !part.trim().is_empty() {
            out.push(part.trim().to_string());
        }
        position = end;
    }
    out
}
fn search_terms(query: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the",
        "and",
        "what",
        "with",
        "from",
        "this",
        "that",
        "have",
        "your",
        "would",
        "about",
        "into",
        "does",
        "there",
        "they",
        "which",
        "where",
        "when",
        "using",
        "please",
        "character",
        "action",
        "recorded",
        "roll",
        "not",
        "for",
        "but",
        "are",
        "can",
        "you",
        "how",
        "has",
        "was",
        "its",
        "his",
        "her",
        "all",
        "any",
        "also",
    ];
    let mut out = Vec::new();
    for term in query
        .split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
    {
        if term.chars().count() > 2
            && term.chars().count() <= 40
            && !STOP.contains(&term.as_str())
            && !out.contains(&term)
        {
            out.push(term);
            if out.len() == 12 {
                break;
            }
        }
    }
    out
}
fn edition_text(profile: RulesProfile) -> &'static str {
    match profile {
        RulesProfile::Manual => "",
        RulesProfile::Legacy2014 => "5e-2014",
        RulesProfile::Revised2024 => "5e-2024",
    }
}

impl CampaignStore {
    pub fn import_knowledge(
        &self,
        campaign_id: &str,
        request: KnowledgeImport,
    ) -> Result<Saved<KnowledgeSource>, EngineError> {
        self.import_knowledge_with_original(campaign_id, request, None)
    }
    pub fn import_knowledge_with_original(
        &self,
        campaign_id: &str,
        request: KnowledgeImport,
        original: Option<OriginalDocumentImport>,
    ) -> Result<Saved<KnowledgeSource>, EngineError> {
        let original = original.map(document_reader::decode_original).transpose()?;
        let title = validate_name(&request.title, "Source title")?;
        if request.text.trim().is_empty()
            || request.text.len() > 8_000_000
            || request.source_ref.len() > 1000
            || !matches!(
                request.edition.as_str(),
                "" | "unassigned" | "5e-2014" | "5e-2024" | "AD&D-2e" | "homebrew"
            )
        {
            return Err(EngineError::Invalid(
                "Import a text source up to 8 MB with a source label and supported edition.".into(),
            ));
        }
        let parts = chunks(&request.text);
        if parts.len() > 6000 {
            return Err(EngineError::Invalid(
                "Split this source into smaller volumes before importing.".into(),
            ));
        }
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let tx = conn.transaction()?;
        let (count,total):(i64,i64)=tx.query_row("SELECT COUNT(*),COALESCE(SUM(byte_count),0) FROM knowledge_sources WHERE campaign_id=?1",[campaign_id],|r|Ok((r.get(0)?,r.get(1)?)))?;
        if count >= 200 || total + request.text.len() as i64 > 150_000_000 {
            return Err(EngineError::Invalid(
                "This campaign supports 200 story/reference sources and 150 MB of text.".into(),
            ));
        }
        let source = KnowledgeSource {
            id: Uuid::new_v4().to_string(),
            title,
            source_ref: request.source_ref,
            edition: request.edition,
            gm_only: request.gm_only,
            chunk_count: parts.len() as i64,
            imported_at: now(),
        };
        tx.execute("INSERT INTO knowledge_sources(id,campaign_id,title,source_ref,edition,gm_only,chunk_count,byte_count,imported_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![source.id,campaign_id,source.title,source.source_ref,source.edition,source.gm_only,source.chunk_count,request.text.len() as i64,source.imported_at])?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO knowledge_chunks(id,source_id,ordinal,body) VALUES (?1,?2,?3,?4)",
            )?;
            for (index, part) in parts.iter().enumerate() {
                stmt.execute(params![
                    Uuid::new_v4().to_string(),
                    source.id,
                    index as i64,
                    part
                ])?;
            }
        }
        if let Some(original) = &original {
            document_reader::save_original(&tx, &source.id, original)?;
        }
        record_event(
            &tx,
            campaign_id,
            "knowledge.imported",
            &json!({"sourceId":source.id,"title":source.title,"chunks":source.chunk_count,"gmOnly":source.gm_only}),
            &source.imported_at,
        )?;
        tx.commit()?;
        Ok(saved(source, &dir, &conn, campaign_id))
    }
    pub fn list_knowledge_sources(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<KnowledgeSource>, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let mut stmt=conn.prepare("SELECT id,title,source_ref,edition,gm_only,chunk_count,imported_at FROM knowledge_sources WHERE campaign_id=?1 ORDER BY title")?;
        let rows = stmt.query_map([campaign_id], |r| {
            Ok(KnowledgeSource {
                id: r.get(0)?,
                title: r.get(1)?,
                source_ref: r.get(2)?,
                edition: r.get(3)?,
                gm_only: r.get(4)?,
                chunk_count: r.get(5)?,
                imported_at: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    pub fn search_dm_references(
        &self,
        campaign_id: &str,
        query: &str,
    ) -> Result<Vec<ReferenceHit>, EngineError> {
        if query.chars().count() > 500 {
            return Err(EngineError::Invalid(
                "Use a reference search up to 500 characters.".into(),
            ));
        }
        let terms = search_terms(query);
        if terms.is_empty() {
            return Ok(vec![]);
        }
        let profile = self.load_rules_settings(campaign_id)?.config.profile;
        let edition = edition_text(profile);
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let expression = terms
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(" OR ");
        let mut stmt=conn.prepare("SELECT reference_id,title,kind,book,edition,gm_only,source_ref,snippet(dm_reference_index,4,'','',' … ',40) FROM dm_reference_index WHERE dm_reference_index MATCH ?1 AND campaign_id=?2 AND (kind='campaign' OR ?3='' OR edition IN ('','unassigned','homebrew') OR edition=?3) ORDER BY bm25(dm_reference_index) LIMIT 6")?;
        let rows = stmt.query_map(params![expression, campaign_id, edition], |r| {
            Ok(ReferenceHit {
                id: r.get(0)?,
                title: r.get(1)?,
                kind: r.get(2)?,
                book: r.get(3)?,
                edition: r.get(4)?,
                gm_only: r.get(5)?,
                source_ref: r.get(6)?,
                excerpt: r.get(7)?,
            })
        })?;
        let mut hits = rows.collect::<Result<Vec<_>, _>>()?;
        // The shipped SRD is always searchable offline for its matching edition.
        if profile == RulesProfile::Revised2024 || profile == RulesProfile::Manual {
            let mut pages = srd_pages()
                .iter()
                .filter_map(|page| {
                    let text = page["text"].as_str()?;
                    let lower = text.to_lowercase();
                    let score = terms
                        .iter()
                        .map(|t| lower.matches(t).count().min(6))
                        .sum::<usize>();
                    if score == 0 {
                        return None;
                    }
                    Some((score, page["number"].as_u64().unwrap_or(0), text))
                })
                .collect::<Vec<_>>();
            pages.sort_by_key(|p| std::cmp::Reverse(p.0));
            for (_, number, text) in pages.into_iter().take(2) {
                let lower = text.to_lowercase();
                let start = terms
                    .iter()
                    .filter_map(|t| lower.find(t))
                    .min()
                    .unwrap_or(0);
                let start = text.floor_char_boundary(start.saturating_sub(120));
                hits.push(ReferenceHit {
                    id: format!("srd:5.2.1:{number}"),
                    title: format!("SRD 5.2.1, page {number}"),
                    kind: "rules".into(),
                    book: "SRD 5.2.1".into(),
                    edition: "5e-2024".into(),
                    gm_only: false,
                    source_ref: "Bundled SRD 5.2.1 (CC-BY-4.0)".into(),
                    excerpt: clip(&text[start..], 1200),
                });
            }
        }
        for hit in &mut hits {
            hit.excerpt = clip(&hit.excerpt, 1200);
        }
        Ok(hits)
    }
    pub fn read_dm_reference(&self, campaign_id: &str, id: &str) -> Result<Value, EngineError> {
        self.read_dm_reference_page(campaign_id, id, 0)
    }
    pub fn read_dm_reference_page(
        &self,
        campaign_id: &str,
        id: &str,
        offset: u32,
    ) -> Result<Value, EngineError> {
        if offset > 100_000_000 {
            return Err(EngineError::Invalid(
                "Reference offset is too large.".into(),
            ));
        }
        // Locate first even for bundled references, to enforce campaign identity.
        let conn = connect(&self.locate(campaign_id)?, false)?;
        if let Some(page) = id
            .strip_prefix("srd:5.2.1:")
            .and_then(|s| s.parse::<u64>().ok())
        {
            let profile = self.load_rules_settings(campaign_id)?.config.profile;
            if profile == RulesProfile::Legacy2014 {
                return Err(EngineError::Invalid(
                    "The bundled SRD is revised 2024 material; this campaign uses 2014.".into(),
                ));
            }
            if let Some(p) = srd_pages()
                .iter()
                .find(|p| p["number"].as_u64() == Some(page))
            {
                return Ok(
                    json!({"id":id,"source":"SRD 5.2.1","page":page,"edition":"5e-2024","text":p["text"].as_str().unwrap_or_default().chars().skip(offset as usize).take(4000).collect::<String>(),"offset":offset,"nextOffset":(offset as usize+4000<p["text"].as_str().unwrap_or_default().chars().count()).then_some(offset+4000),"gmOnly":false}),
                );
            }
        }
        if Uuid::parse_str(id).is_err() {
            return Err(EngineError::Invalid(
                "Choose a reference ID returned by the search.".into(),
            ));
        }
        let row:Option<(String,String,String,String,bool,String,String)>=conn.query_row("SELECT title,body,book,edition,gm_only,source_ref,kind FROM dm_reference_index WHERE campaign_id=?1 AND reference_id=?2",params![campaign_id,id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional()?;
        let (title, body, book, edition, gm_only, source_ref, kind) = row
            .ok_or_else(|| EngineError::Invalid("Reference not found in this campaign.".into()))?;
        let profile = self.load_rules_settings(campaign_id)?.config.profile;
        let active = edition_text(profile);
        if kind != "campaign"
            && !active.is_empty()
            && matches!(edition.as_str(), "5e-2014" | "5e-2024" | "AD&D-2e")
            && edition != active
        {
            return Err(EngineError::Invalid(
                "This reference belongs to another rules edition.".into(),
            ));
        }
        Ok(
            json!({"id":id,"title":title,"text":body.chars().skip(offset as usize).take(4000).collect::<String>(),"offset":offset,"nextOffset":(offset as usize+4000<body.chars().count()).then_some(offset+4000),"book":book,"edition":edition,"gmOnly":gm_only,"sourceRef":source_ref,"truncated":offset as usize+4000<body.chars().count(),"rulesWarning":"Source editions are reference metadata. Apply mechanics from the active rules profile; setting lore is not a rules override."}),
        )
    }
}
fn srd_pages() -> &'static Vec<Value> {
    static PAGES: OnceLock<Vec<Value>> = OnceLock::new();
    PAGES.get_or_init(|| {
        serde_json::from_str::<Value>(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../apps/vtt-ui/public/rules/srd-5.2.1.json"
        )))
        .expect("bundled SRD JSON")["pages"]
            .as_array()
            .expect("SRD pages")
            .clone()
    })
}
