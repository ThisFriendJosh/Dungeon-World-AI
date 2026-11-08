use super::*;
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum CalculationOperation {
    Arithmetic { expression: String },
    Rules { query: RulesQuery },
    Damage { query: DamageQuery },
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CalculationRequest {
    pub character_id: Option<String>,
    pub operation: CalculationOperation,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalculationRecord {
    pub id: String,
    pub kind: String,
    pub character_id: Option<String>,
    pub inputs: Value,
    pub result: Value,
    pub preview_only: bool,
    pub event_seq: i64,
    pub created_at: String,
}
impl CampaignStore {
    pub fn calculate_campaign(
        &self,
        campaign_id: &str,
        request: CalculationRequest,
    ) -> Result<Saved<CalculationRecord>, EngineError> {
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let before: i64 = conn.query_row(
            "SELECT COALESCE(MAX(seq),0) FROM events WHERE campaign_id=?1",
            [campaign_id],
            |r| r.get(0),
        )?;
        if let Some(id) = &request.character_id {
            let valid: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM characters WHERE campaign_id=?1 AND id=?2)",
                params![campaign_id, id],
                |r| r.get(0),
            )?;
            if !valid {
                return Err(EngineError::CharacterNotFound(id.clone()));
            }
        }
        let inputs = serde_json::to_value(&request.operation)
            .map_err(|e| EngineError::Invalid(e.to_string()))?;
        let (kind, result) = match request.operation {
            CalculationOperation::Arithmetic { expression } => {
                ("arithmetic", json!(calculate_game_math(&expression)?))
            }
            CalculationOperation::Rules { query } => (
                "rules",
                serde_json::to_value(self.calculate_rules(campaign_id, query)?)
                    .map_err(|e| EngineError::Invalid(e.to_string()))?,
            ),
            CalculationOperation::Damage { query } => (
                "damage",
                serde_json::to_value(calculate_damage(query)?)
                    .map_err(|e| EngineError::Invalid(e.to_string()))?,
            ),
        };
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let current: i64 = tx.query_row(
            "SELECT COALESCE(MAX(seq),0) FROM events WHERE campaign_id=?1",
            [campaign_id],
            |r| r.get(0),
        )?;
        if current != before {
            return Err(EngineError::Invalid(
                "The campaign changed while calculating. Retry with its current values.".into(),
            ));
        }
        let id = Uuid::new_v4().to_string();
        let created_at = now();
        record_event(
            &tx,
            campaign_id,
            "calculation.recorded",
            &json!({"id":id,"kind":kind,"characterId":request.character_id,"previewOnly":true}),
            &created_at,
        )?;
        let event_seq = tx.last_insert_rowid();
        tx.execute("INSERT INTO campaign_calculations(id,campaign_id,kind,character_id,inputs_json,result_json,event_seq,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",params![id,campaign_id,kind,request.character_id,inputs.to_string(),result.to_string(),event_seq,created_at])?;
        tx.commit()?;
        Ok(saved(
            CalculationRecord {
                id,
                kind: kind.into(),
                character_id: request.character_id,
                inputs,
                result,
                preview_only: true,
                event_seq,
                created_at,
            },
            &dir,
            &conn,
            campaign_id,
        ))
    }
    pub fn list_calculations(
        &self,
        campaign_id: &str,
        offset: u32,
    ) -> Result<Vec<CalculationRecord>, EngineError> {
        if offset > 100_000 {
            return Err(EngineError::Invalid(
                "Choose a calculation history page within range.".into(),
            ));
        }
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let mut stmt=conn.prepare("SELECT id,kind,character_id,inputs_json,result_json,event_seq,created_at FROM campaign_calculations c WHERE campaign_id=?1 AND NOT EXISTS(SELECT 1 FROM event_visibility v WHERE v.campaign_id=c.campaign_id AND v.event_seq=c.event_seq AND v.removed=1) ORDER BY event_seq DESC LIMIT 20 OFFSET ?2")?;
        let rows = stmt
            .query_map(params![campaign_id, offset], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, String>(6)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(
                |(id, kind, character_id, inputs, result, event_seq, created_at)| {
                    Ok(CalculationRecord {
                        id,
                        kind,
                        character_id,
                        inputs: serde_json::from_str(&inputs)
                            .map_err(|e| EngineError::Invalid(e.to_string()))?,
                        result: serde_json::from_str(&result)
                            .map_err(|e| EngineError::Invalid(e.to_string()))?,
                        preview_only: true,
                        event_seq,
                        created_at,
                    })
                },
            )
            .collect()
    }
}
