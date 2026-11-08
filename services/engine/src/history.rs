use super::*;
use rusqlite::TransactionBehavior;
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    #[serde(flatten)]
    pub event: CampaignEvent,
    pub removed: bool,
    pub can_remove: bool,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryEdit {
    pub event_seq: i64,
    pub expected_seq: i64,
    pub removed: bool,
    pub confirmed: bool,
}
impl CampaignStore {
    pub fn load_editable_history(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<HistoryEntry>, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let mut statement=conn.prepare("SELECT e.seq,e.event_type,e.payload_json,e.created_at,COALESCE(v.removed,0) FROM events e LEFT JOIN event_visibility v ON v.campaign_id=e.campaign_id AND v.event_seq=e.seq WHERE e.campaign_id=?1 ORDER BY e.seq DESC LIMIT 200")?;
        let rows = statement.query_map([campaign_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, bool>(4)?,
            ))
        })?;
        rows.map(|row| {
            let (seq, kind, payload, created_at, removed) = row?;
            Ok(HistoryEntry {
                can_remove: !kind.starts_with("history."),
                removed,
                event: CampaignEvent {
                    seq,
                    event_type: kind,
                    payload: serde_json::from_str(&payload)
                        .map_err(|e| EngineError::Invalid(e.to_string()))?,
                    created_at,
                },
            })
        })
        .collect()
    }

    /// Remove a single entry from the visible timeline and DM memory. Original
    /// canonical events stay recoverable; this does not undo current game state.
    pub fn edit_history_entry(
        &self,
        campaign_id: &str,
        request: HistoryEdit,
    ) -> Result<Saved<Value>, EngineError> {
        if !request.confirmed || request.event_seq < 1 || request.expected_seq < 1 {
            return Err(EngineError::Invalid(
                "Select one history entry and confirm the change.".into(),
            ));
        }
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let latest: i64 = tx.query_row(
            "SELECT COALESCE(MAX(seq),0) FROM events WHERE campaign_id=?1",
            [campaign_id],
            |r| r.get(0),
        )?;
        if latest != request.expected_seq {
            return Err(EngineError::Invalid(
                "The campaign changed. Refresh and select the history entry again.".into(),
            ));
        }
        let kind: Option<String> = tx
            .query_row(
                "SELECT event_type FROM events WHERE campaign_id=?1 AND seq=?2",
                params![campaign_id, request.event_seq],
                |r| r.get(0),
            )
            .optional()?;
        let kind = kind.ok_or_else(|| {
            EngineError::Invalid("History entry not found in this campaign.".into())
        })?;
        if kind.starts_with("history.") {
            return Err(EngineError::Invalid(
                "History edit records are retained for recovery.".into(),
            ));
        }
        let ts = now();
        tx.execute("INSERT INTO event_visibility(campaign_id,event_seq,removed,updated_at) VALUES (?1,?2,?3,?4) ON CONFLICT(campaign_id,event_seq) DO UPDATE SET removed=excluded.removed,updated_at=excluded.updated_at",params![campaign_id,request.event_seq,request.removed,ts])?;
        let value = json!({"eventSeq":request.event_seq,"removed":request.removed,"currentStateUnchanged":true});
        record_event(
            &tx,
            campaign_id,
            if request.removed {
                "history.entry.removed"
            } else {
                "history.entry.restored"
            },
            &value,
            &ts,
        )?;
        tx.commit()?;
        Ok(saved(value, &dir, &conn, campaign_id))
    }
}
