use super::*;
use rusqlite::TransactionBehavior;
use serde_json::json;

/// Deliberately contains no arbitrary events or mechanical write operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DmProposal {
    pub schema_version: u32,
    pub narration: String,
    pub choices: Vec<String>,
}

impl DmProposal {
    pub fn validate(&self) -> Result<(), EngineError> {
        if self.schema_version != 1
            || self.narration.trim().is_empty()
            || self.narration.chars().count() > 6000
            || self.choices.len() > 4
            || self
                .choices
                .iter()
                .any(|s| s.trim().is_empty() || s.chars().count() > 240)
            || self.narration.contains("<think>")
            || self.narration.contains("</think>")
        {
            return Err(EngineError::Invalid("The DM returned an invalid or unfinished reply. Your campaign has not changed; retry your action.".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DmTurnRequest {
    pub request_id: String,
    pub actor_id: Option<String>,
    pub player_text: String,
    pub expected_seq: i64,
}

impl DmTurnRequest {
    fn validate(&self) -> Result<(), EngineError> {
        if Uuid::parse_str(&self.request_id).is_err()
            || self
                .actor_id
                .as_ref()
                .is_some_and(|s| Uuid::parse_str(s).is_err())
            || self.player_text.trim().is_empty()
            || self.player_text.chars().count() > 4000
            || self.expected_seq < 1
        {
            return Err(EngineError::Invalid(
                "Enter an action of 1–4000 characters and refresh the campaign before retrying."
                    .into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmTurn {
    pub id: String,
    pub campaign_id: String,
    pub request_id: String,
    pub actor_id: Option<String>,
    pub player_text: String,
    pub reply: DmProposal,
    pub model: String,
    pub prompt_version: String,
    pub based_on_seq: i64,
    pub event_seq: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmHistory {
    pub turns: Vec<DmTurn>,
    pub total_turns: i64,
    pub latest_event_seq: i64,
}

/// Engine-owned snapshot with explicit GM-only knowledge and player-facing facts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmContext {
    pub campaign_id: String,
    pub state_seq: i64,
    pub state: Value,
    pub history: Vec<DmTurn>,
    pub total_turns: i64,
}

fn latest_seq(conn: &Connection, campaign_id: &str) -> Result<i64, EngineError> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(seq),0) FROM events WHERE campaign_id=?1",
        [campaign_id],
        |r| r.get(0),
    )?)
}

fn turn_row(r: &Row<'_>) -> rusqlite::Result<DmTurn> {
    let raw: String = r.get(5)?;
    let reply = serde_json::from_str(&raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(DmTurn {
        id: r.get(0)?,
        campaign_id: r.get(1)?,
        request_id: r.get(2)?,
        actor_id: r.get(3)?,
        player_text: r.get(4)?,
        reply,
        model: r.get(6)?,
        prompt_version: r.get(7)?,
        based_on_seq: r.get(8)?,
        event_seq: r.get(9)?,
        created_at: r.get(10)?,
    })
}

const COLUMNS: &str = "id,campaign_id,request_id,actor_id,player_text,proposal_json,model,prompt_version,based_on_seq,event_seq,created_at";

fn history(conn: &Connection, campaign_id: &str, limit: i64) -> Result<DmHistory, EngineError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM dm_turns t WHERE campaign_id=?1 AND NOT EXISTS(SELECT 1 FROM event_visibility v WHERE v.campaign_id=t.campaign_id AND v.event_seq=t.event_seq AND v.removed=1) ORDER BY event_seq DESC LIMIT ?2"
    ))?;
    let mut turns = stmt
        .query_map(params![campaign_id, limit], turn_row)?
        .collect::<Result<Vec<_>, _>>()?;
    turns.reverse();
    let total_turns = conn.query_row(
        "SELECT COUNT(*) FROM dm_turns t WHERE campaign_id=?1 AND NOT EXISTS(SELECT 1 FROM event_visibility v WHERE v.campaign_id=t.campaign_id AND v.event_seq=t.event_seq AND v.removed=1)",
        [campaign_id],
        |r| r.get(0),
    )?;
    Ok(DmHistory {
        turns,
        total_turns,
        latest_event_seq: latest_seq(conn, campaign_id)?,
    })
}

fn existing(
    conn: &Connection,
    campaign_id: &str,
    request: &DmTurnRequest,
) -> Result<Option<DmTurn>, EngineError> {
    let turn = conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM dm_turns WHERE campaign_id=?1 AND request_id=?2"),
            params![campaign_id, request.request_id],
            turn_row,
        )
        .optional()?;
    if let Some(ref t) = turn {
        let removed:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM event_visibility WHERE campaign_id=?1 AND event_seq=?2 AND removed=1)",params![campaign_id,t.event_seq],|r|r.get(0))?;
        if removed {
            return Err(EngineError::Invalid(
                "This action was removed from history. Send it again as a new action.".into(),
            ));
        }
        if t.player_text != request.player_text.trim() || t.actor_id != request.actor_id {
            return Err(EngineError::Invalid(
                "This request identifier belongs to a different action.".into(),
            ));
        }
    }
    Ok(turn)
}

fn short(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn public_items(raw: &str, count: usize) -> Vec<String> {
    serde_json::from_str::<Vec<Value>>(raw)
        .unwrap_or_default()
        .iter()
        .take(count)
        .map(|v| {
            short(
                v.as_str()
                    .unwrap_or("[structured item; details require a table ruling]"),
                160,
            )
        })
        .collect()
}

impl CampaignStore {
    pub fn load_dm_history(&self, campaign_id: &str) -> Result<DmHistory, EngineError> {
        let mut conn = connect(&self.locate(campaign_id)?, false)?;
        let tx = conn.transaction()?;
        history(&tx, campaign_id, 50)
    }

    pub fn find_dm_turn(
        &self,
        campaign_id: &str,
        request: &DmTurnRequest,
    ) -> Result<Option<DmTurn>, EngineError> {
        request.validate()?;
        existing(
            &connect(&self.locate(campaign_id)?, false)?,
            campaign_id,
            request,
        )
    }

    pub fn prepare_dm_context(
        &self,
        campaign_id: &str,
        request: &DmTurnRequest,
    ) -> Result<DmContext, EngineError> {
        request.validate()?;
        // These readers use separate connections; revision checks bracket the reads so
        // both local and other-process writes invalidate a mixed snapshot.
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let before = latest_seq(&conn, campaign_id)?;
        if before != request.expected_seq {
            return Err(EngineError::Invalid("The campaign changed. Refresh it before sending this action; your draft is still available.".into()));
        }
        let characters = self.load_characters(campaign_id)?;
        if request
            .actor_id
            .as_ref()
            .is_some_and(|id| !characters.iter().any(|c| &c.id == id))
        {
            return Err(EngineError::Invalid(
                "Choose a character from this campaign.".into(),
            ));
        }
        if characters.len() > 12 {
            return Err(EngineError::Invalid(
                "The first DM version supports up to 12 campaign characters.".into(),
            ));
        }
        let party: Vec<Value> = characters.iter().map(|c| {
            let stats: Value = serde_json::from_str(&c.stats_json).unwrap_or(json!({}));
            let mut safe_stats = serde_json::Map::new();
            for key in ["str","dex","con","int","wis","cha","hp","maxHp","armorClass"] {
                if let Some(n) = stats.get(key).and_then(Value::as_i64) { safe_stats.insert(key.into(), json!(n)); }
            }
            json!({"id":c.id,"name":c.name,"class":c.class_name,"level":c.level,"recordedStats":safe_stats,
                "inventory":public_items(&c.inventory_json,24),"conditions":public_items(&c.conditions_json,12),"derived":dm_tools::derived_character(c)})
        }).collect();
        let tabletop = self.load_tabletop(campaign_id)?;
        // The DM may inspect the whole saved table. Player knowledge and GM-only
        // material remain explicitly labeled; raw historical scene events stay out.
        let scene = tabletop.scene.as_ref().map(|s| json!({"name":s.name,
            "publicCards":s.index_cards.public.iter().take(8).map(|c| json!({"title":c.title,
                "text":c.blocks.first().map(|b|short(&b.value,512)).unwrap_or_default()})).collect::<Vec<_>>() }));
        let rules = self.load_rules_settings(campaign_id)?;
        let gm_context=tabletop.scene.as_ref().map(|s|json!({"gmKnowledgeOnly":true,"sceneId":s.id,
            "sceneNotes":short(&s.notes,1800),"privateCards":s.index_cards.private.iter().take(12).map(|c|json!({"id":c.id,"title":c.title,"text":c.blocks.first().map(|b|short(&b.value,700)).unwrap_or_default()})).collect::<Vec<_>>(),
            "board":s.board.as_ref().map(dm_tools::board_snapshot)}));
        let references = self
            .search_dm_references(campaign_id, &short(&request.player_text, 500))?
            .into_iter()
            .take(4)
            .collect::<Vec<_>>();
        let sources = self.list_knowledge_sources(campaign_id)?;
        let scenes = self.list_tabletop_scenes(campaign_id)?;
        let rolls = self
            .load_dice_rolls(campaign_id)?
            .into_iter()
            .take(6)
            .collect::<Vec<_>>();
        let campaign_name: String = conn.query_row(
            "SELECT name FROM campaigns WHERE id=?1",
            [campaign_id],
            |r| r.get(0),
        )?;
        let history = history(&conn, campaign_id, 12)?;
        if latest_seq(&conn, campaign_id)? != before {
            return Err(EngineError::Invalid(
                "The campaign changed while reading its context. Refresh and retry.".into(),
            ));
        }
        Ok(DmContext {
            campaign_id: campaign_id.into(),
            state_seq: before,
            state: json!({"campaign":campaign_name,"actingCharacterId":request.actor_id,"party":party,
                "rules":rules,"scene":scene,"gmContext":gm_context,"sceneCatalog":scenes,"recordedDiceRolls":rolls,
                "referenceMatches":references,"knowledgeSources":sources.iter().take(24).collect::<Vec<_>>(),"knowledgeSourceCount":sources.len(),
                "scope":"DM can inspect all saved table/character data, search local campaign sources/compendium/SRD, and request engine calculations through read tools. GM-only knowledge must not appear in narration until discovered. Mechanical changes remain manual. A die total alone does not establish success, failure, or a critical. Armor Class is a total, not an attack bonus."}),
            history: history.turns,
            total_turns: history.total_turns,
        })
    }

    pub fn commit_dm_turn(
        &self,
        campaign_id: &str,
        request: &DmTurnRequest,
        proposal: DmProposal,
        model: &str,
        prompt_version: &str,
    ) -> Result<Saved<DmTurn>, EngineError> {
        request.validate()?;
        proposal.validate()?;
        if model.trim().is_empty()
            || model.chars().count() > 160
            || prompt_version.trim().is_empty()
            || prompt_version.len() > 80
        {
            return Err(EngineError::Invalid("Invalid DM provenance.".into()));
        }
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(turn) = existing(&tx, campaign_id, request)? {
            drop(tx);
            return Ok(saved(turn, &dir, &conn, campaign_id));
        }
        if latest_seq(&tx, campaign_id)? != request.expected_seq {
            return Err(EngineError::Invalid("The campaign changed during generation. The draft reply was discarded. Refresh and retry your action.".into()));
        }
        if let Some(id) = &request.actor_id {
            let found: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM characters WHERE id=?1 AND campaign_id=?2)",
                params![id, campaign_id],
                |r| r.get(0),
            )?;
            if !found {
                return Err(EngineError::CharacterNotFound(id.clone()));
            }
        }
        let id = Uuid::new_v4().to_string();
        let timestamp = now();
        record_event(
            &tx,
            campaign_id,
            "dm.turn.saved",
            &json!({"turnId":id,"requestId":request.request_id,"actorId":request.actor_id,"basedOnSeq":request.expected_seq,"model":model,"promptVersion":prompt_version}),
            &timestamp,
        )?;
        let event_seq = tx.last_insert_rowid();
        let turn = DmTurn {
            id,
            campaign_id: campaign_id.into(),
            request_id: request.request_id.clone(),
            actor_id: request.actor_id.clone(),
            player_text: request.player_text.trim().into(),
            reply: proposal,
            model: model.into(),
            prompt_version: prompt_version.into(),
            based_on_seq: request.expected_seq,
            event_seq,
            created_at: timestamp,
        };
        tx.execute("INSERT INTO dm_turns(id,campaign_id,request_id,actor_id,player_text,proposal_json,model,prompt_version,based_on_seq,event_seq,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![turn.id,campaign_id,turn.request_id,turn.actor_id,turn.player_text,serde_json::to_string(&turn.reply).map_err(|e|EngineError::Invalid(e.to_string()))?,model,prompt_version,turn.based_on_seq,event_seq,turn.created_at])?;
        tx.commit()?;
        Ok(saved(turn, &dir, &conn, campaign_id))
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod workspace_tests;
