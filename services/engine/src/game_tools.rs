use super::*;
use serde_json::json;
const MAX_MS: u64 = 315_360_000_000;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameTimer {
    pub id: String,
    pub label: String,
    pub clock: String,
    pub mode: String,
    pub duration_ms: u64,
    pub value_ms: u64,
    pub running: bool,
    pub started_at_ms: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolsState {
    pub game_time_ms: u64,
    pub timers: Vec<GameTimer>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameTools {
    pub state: ToolsState,
    pub revision: i64,
    pub server_now_ms: i64,
    pub updated_at: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TimerOperation {
    Add {
        label: String,
        clock: String,
        mode: String,
        seconds: u64,
    },
    Start {
        id: String,
    },
    Pause {
        id: String,
    },
    Reset {
        id: String,
    },
    Remove {
        id: String,
        confirmed: bool,
    },
    Advance {
        seconds: u64,
    },
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolMutation {
    pub expected_revision: i64,
    pub operation: TimerOperation,
}
pub(crate) fn timer_value(timer: &GameTimer, at: i64) -> u64 {
    let elapsed = if timer.running && timer.clock == "real" {
        at.saturating_sub(timer.started_at_ms.unwrap_or(at)).max(0) as u64
    } else {
        0
    };
    if timer.mode == "countdown" {
        timer.value_ms.saturating_sub(elapsed)
    } else {
        timer.value_ms.saturating_add(elapsed).min(MAX_MS)
    }
}
fn normalize(state: &mut ToolsState, at: i64) {
    for timer in &mut state.timers {
        timer.value_ms = timer_value(timer, at);
        if timer.mode == "countdown" && timer.value_ms == 0 {
            timer.running = false;
        }
        timer.started_at_ms = if timer.running && timer.clock == "real" {
            Some(at)
        } else {
            None
        };
    }
}
fn apply(state: &mut ToolsState, op: &TimerOperation, at: i64) -> Result<(), EngineError> {
    normalize(state, at);
    match op {
        TimerOperation::Add {
            label,
            clock,
            mode,
            seconds,
        } => {
            let label = validate_name(label, "Timer label")?;
            if state.timers.len() >= 20
                || label.chars().any(char::is_control)
                || !matches!(clock.as_str(), "real" | "game")
                || !matches!(mode.as_str(), "countdown" | "stopwatch")
                || *seconds > 31_536_000
                || mode == "countdown" && *seconds == 0
            {
                return Err(EngineError::Invalid(
                    "Use up to 20 timers, with a duration from 1 second to one year.".into(),
                ));
            }
            let duration_ms = if mode == "stopwatch" {
                0
            } else {
                seconds * 1000
            };
            state.timers.push(GameTimer {
                id: Uuid::new_v4().to_string(),
                label,
                clock: clock.clone(),
                mode: mode.clone(),
                duration_ms,
                value_ms: duration_ms,
                running: false,
                started_at_ms: None,
            });
        }
        TimerOperation::Advance { seconds } => {
            if !(1..=86400).contains(seconds) {
                return Err(EngineError::Invalid(
                    "Advance game time by 1 second to one day at a time.".into(),
                ));
            }
            let ms = seconds * 1000;
            state.game_time_ms = state
                .game_time_ms
                .checked_add(ms)
                .filter(|v| *v <= MAX_MS)
                .ok_or_else(|| {
                    EngineError::Invalid("The game clock has reached its limit.".into())
                })?;
            for timer in &mut state.timers {
                if timer.clock == "game" && timer.running {
                    timer.value_ms = if timer.mode == "countdown" {
                        timer.value_ms.saturating_sub(ms)
                    } else {
                        timer.value_ms.saturating_add(ms).min(MAX_MS)
                    };
                    if timer.mode == "countdown" && timer.value_ms == 0 {
                        timer.running = false;
                    }
                }
            }
        }
        TimerOperation::Remove { id, confirmed } => {
            if !confirmed {
                return Err(EngineError::Invalid(
                    "Confirm removal of this timer.".into(),
                ));
            }
            let index = state
                .timers
                .iter()
                .position(|t| &t.id == id)
                .ok_or_else(|| EngineError::Invalid("Timer not found in this campaign.".into()))?;
            state.timers.remove(index);
        }
        operation => {
            let id = match operation {
                TimerOperation::Start { id }
                | TimerOperation::Pause { id }
                | TimerOperation::Reset { id } => id,
                _ => unreachable!(),
            };
            let timer = state
                .timers
                .iter_mut()
                .find(|t| &t.id == id)
                .ok_or_else(|| EngineError::Invalid("Timer not found in this campaign.".into()))?;
            match operation {
                TimerOperation::Start { .. } => {
                    if timer.mode == "countdown" && timer.value_ms == 0 {
                        return Err(EngineError::Invalid(
                            "Reset this completed timer before starting it.".into(),
                        ));
                    }
                    timer.running = true;
                    timer.started_at_ms = if timer.clock == "real" {
                        Some(at)
                    } else {
                        None
                    };
                }
                TimerOperation::Pause { .. } => {
                    timer.running = false;
                    timer.started_at_ms = None;
                }
                TimerOperation::Reset { .. } => {
                    timer.value_ms = timer.duration_ms;
                    timer.running = false;
                    timer.started_at_ms = None;
                }
                _ => unreachable!(),
            }
        }
    }
    Ok(())
}
fn read(
    conn: &Connection,
    campaign_id: &str,
) -> Result<(ToolsState, i64, Option<String>), EngineError> {
    let row: Option<(String, i64, String)> = conn
        .query_row(
            "SELECT state_json,revision,updated_at FROM game_tools WHERE campaign_id=?1",
            [campaign_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    match row {
        Some((text, revision, at)) => Ok((
            serde_json::from_str(&text).map_err(|e| EngineError::Invalid(e.to_string()))?,
            revision,
            Some(at),
        )),
        None => Ok((ToolsState::default(), 0, None)),
    }
}
impl CampaignStore {
    pub fn load_game_tools(&self, campaign_id: &str) -> Result<GameTools, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let (mut state, revision, updated_at) = read(&conn, campaign_id)?;
        let server_now_ms = Utc::now().timestamp_millis();
        normalize(&mut state, server_now_ms);
        Ok(GameTools {
            state,
            revision,
            updated_at,
            server_now_ms,
        })
    }
    pub fn change_game_tools(
        &self,
        campaign_id: &str,
        request: ToolMutation,
    ) -> Result<Saved<GameTools>, EngineError> {
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let tx = conn.transaction()?;
        let (mut state, revision, _) = read(&tx, campaign_id)?;
        if request.expected_revision != revision {
            return Err(EngineError::Invalid(
                "Timers changed in another window. Refresh before trying again.".into(),
            ));
        }
        let at = Utc::now().timestamp_millis();
        apply(&mut state, &request.operation, at)?;
        let revision = revision
            .checked_add(1)
            .ok_or_else(|| EngineError::Invalid("Timer revision limit reached.".into()))?;
        let timestamp = now();
        tx.execute("INSERT INTO game_tools(campaign_id,state_json,revision,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(campaign_id) DO UPDATE SET state_json=excluded.state_json,revision=excluded.revision,updated_at=excluded.updated_at",params![campaign_id,serde_json::to_string(&state).unwrap(),revision,timestamp])?;
        record_event(
            &tx,
            campaign_id,
            "tools.changed",
            &json!({"operation":request.operation,"revision":revision,"gameTimeMs":state.game_time_ms}),
            &timestamp,
        )?;
        tx.commit()?;
        Ok(saved(
            GameTools {
                state,
                revision,
                server_now_ms: at,
                updated_at: Some(timestamp),
            },
            &dir,
            &conn,
            campaign_id,
        ))
    }
}

// Small numeric grammar: arithmetic and parentheses only; no executable expressions.
pub fn calculate_game_math(expression: &str) -> Result<f64, EngineError> {
    struct Parser<'a> {
        input: &'a [u8],
        at: usize,
        depth: usize,
    }
    impl Parser<'_> {
        fn space(&mut self) {
            while self.input.get(self.at).is_some_and(u8::is_ascii_whitespace) {
                self.at += 1;
            }
        }
        fn take(&mut self, c: u8) -> bool {
            self.space();
            if self.input.get(self.at) == Some(&c) {
                self.at += 1;
                true
            } else {
                false
            }
        }
        fn sum(&mut self) -> Option<f64> {
            let mut a = self.product()?;
            loop {
                if self.take(b'+') {
                    a += self.product()?;
                } else if self.take(b'-') {
                    a -= self.product()?;
                } else {
                    break;
                }
            }
            Some(a)
        }
        fn product(&mut self) -> Option<f64> {
            let mut a = self.atom()?;
            loop {
                if self.take(b'*') {
                    a *= self.atom()?;
                } else if self.take(b'/') {
                    let b = self.atom()?;
                    if b == 0.0 {
                        return None;
                    }
                    a /= b;
                } else {
                    break;
                }
            }
            Some(a)
        }
        fn atom(&mut self) -> Option<f64> {
            self.space();
            self.depth += 1;
            if self.depth > 24 {
                return None;
            }
            let result = if self.take(b'+') {
                self.atom()
            } else if self.take(b'-') {
                self.atom().map(|n| -n)
            } else if self.take(b'(') {
                let n = self.sum()?;
                if !self.take(b')') {
                    return None;
                }
                Some(n)
            } else {
                let start = self.at;
                while self
                    .input
                    .get(self.at)
                    .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
                {
                    self.at += 1;
                }
                std::str::from_utf8(&self.input[start..self.at])
                    .ok()?
                    .parse::<f64>()
                    .ok()
            };
            self.depth -= 1;
            result.filter(|n| n.is_finite() && n.abs() <= 1e15)
        }
    }
    let bad = || {
        EngineError::Invalid(
            "Use numbers, parentheses, +, −, * and /. Division by zero is undefined.".into(),
        )
    };
    if expression.len() > 256 {
        return Err(bad());
    }
    let normalized = expression
        .replace('−', "-")
        .replace('×', "*")
        .replace('÷', "/")
        .replace('\u{00a0}', " ");
    let mut parser = Parser {
        input: normalized.as_bytes(),
        at: 0,
        depth: 0,
    };
    let result = parser.sum().ok_or_else(bad)?;
    parser.space();
    if parser.at != parser.input.len() || !result.is_finite() || result.abs() > 1e15 {
        return Err(bad());
    }
    Ok(result)
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DamageQuery {
    pub hp: u32,
    pub max_hp: u32,
    pub temp_hp: u32,
    pub amount: u32,
    pub healing: bool,
    pub resistance: bool,
    pub vulnerability: bool,
    pub immune: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DamageResult {
    pub adjusted_amount: u32,
    pub hp_after: u32,
    pub temp_hp_after: u32,
    pub absorbed: u32,
    pub hp_change: i64,
}
pub fn calculate_damage(q: DamageQuery) -> Result<DamageResult, EngineError> {
    if q.max_hp == 0 || q.max_hp > 9999 || q.hp > q.max_hp || q.temp_hp > 9999 || q.amount > 100000
    {
        return Err(EngineError::Invalid(
            "Check HP, maximum HP, temporary HP, and the amount.".into(),
        ));
    }
    let adjusted_amount = if q.healing {
        q.amount
    } else if q.immune {
        0
    } else {
        (if q.resistance { q.amount / 2 } else { q.amount }) * if q.vulnerability { 2 } else { 1 }
    };
    let absorbed = if q.healing {
        0
    } else {
        adjusted_amount.min(q.temp_hp)
    };
    let hp_after = if q.healing {
        q.hp.saturating_add(adjusted_amount).min(q.max_hp)
    } else {
        q.hp.saturating_sub(adjusted_amount - absorbed)
    };
    Ok(DamageResult {
        adjusted_amount,
        hp_after,
        temp_hp_after: q.temp_hp - absorbed,
        absorbed,
        hp_change: hp_after as i64 - q.hp as i64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reading_an_expired_timer_reports_complete_without_writing_events_or_state() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("tools-expiry-test-{}", Uuid::new_v4()));
        let store = CampaignStore::new(path.clone()).unwrap();
        let campaign = store.create_campaign("Expired timer").unwrap().value;
        store
            .change_game_tools(
                &campaign.id,
                ToolMutation {
                    expected_revision: 0,
                    operation: TimerOperation::Add {
                        label: "Turn".into(),
                        clock: "real".into(),
                        mode: "countdown".into(),
                        seconds: 5,
                    },
                },
            )
            .unwrap();
        let conn = connect(&store.locate(&campaign.id).unwrap(), false).unwrap();
        let mut state = store.load_game_tools(&campaign.id).unwrap().state;
        state.timers[0].running = true;
        state.timers[0].started_at_ms = Some(1);
        let encoded = serde_json::to_string(&state).unwrap();
        conn.execute(
            "UPDATE game_tools SET state_json=?2 WHERE campaign_id=?1",
            params![campaign.id, encoded],
        )
        .unwrap();
        let count = store.recent_events(&campaign.id).unwrap().len();
        let result = store.load_game_tools(&campaign.id).unwrap();
        assert_eq!(result.state.timers[0].value_ms, 0);
        assert!(!result.state.timers[0].running);
        assert_eq!(result.revision, 1);
        assert_eq!(store.recent_events(&campaign.id).unwrap().len(), count);
        let persisted: String = conn
            .query_row(
                "SELECT state_json FROM game_tools WHERE campaign_id=?1",
                [&campaign.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(persisted, encoded);
        drop(conn);
        drop(store);
        assert!(path.starts_with(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target")));
        let _ = fs::remove_dir_all(path);
    }
    fn timer(clock: &str, mode: &str) -> GameTimer {
        GameTimer {
            id: "timer".into(),
            label: "Torch".into(),
            clock: clock.into(),
            mode: mode.into(),
            duration_ms: 60000,
            value_ms: 60000,
            running: true,
            started_at_ms: Some(1000),
        }
    }
    #[test]
    fn real_timers_resume_from_absolute_time_and_pause_without_drift() {
        let mut s = ToolsState {
            game_time_ms: 0,
            timers: vec![timer("real", "countdown")],
        };
        assert_eq!(timer_value(&s.timers[0], 16000), 45000);
        apply(&mut s, &TimerOperation::Pause { id: "timer".into() }, 16500).unwrap();
        assert_eq!(timer_value(&s.timers[0], 999999), 44500);
        apply(&mut s, &TimerOperation::Start { id: "timer".into() }, 20000).unwrap();
        assert_eq!(timer_value(&s.timers[0], 20500), 44000);
        assert_eq!(timer_value(&s.timers[0], 19000), 44500);
    }
    #[test]
    fn game_advances_only_tick_running_game_timers() {
        let mut s = ToolsState {
            game_time_ms: 0,
            timers: vec![timer("game", "countdown"), timer("real", "countdown")],
        };
        apply(&mut s, &TimerOperation::Advance { seconds: 6 }, 1000).unwrap();
        assert_eq!(s.game_time_ms, 6000);
        assert_eq!(s.timers[0].value_ms, 54000);
        assert_eq!(s.timers[1].value_ms, 60000);
        apply(&mut s, &TimerOperation::Advance { seconds: 60 }, 1000).unwrap();
        assert!(!s.timers[0].running);
        assert_eq!(s.timers[0].value_ms, 0);
        assert!(s.timers[1].running);
    }
    #[test]
    fn calculator_obeys_precedence_and_rejects_code_zero_division_and_excessive_nesting() {
        assert_eq!(calculate_game_math("10 − 2 × 3").unwrap(), 4.0);
        assert_eq!(calculate_game_math("12 ÷ 3").unwrap(), 4.0);
        assert_eq!(calculate_game_math("-(3+2)*4+21/2").unwrap(), -9.5);
        for e in [
            "1/0",
            "alert(1)",
            "1;2",
            "2**2",
            "2+",
            "NaN",
            "9999999999999999",
        ] {
            assert!(calculate_game_math(e).is_err(), "{e}");
        }
        assert!(calculate_game_math(&("(".repeat(30) + "1" + &")".repeat(30))).is_err());
    }
    #[test]
    fn damage_rounds_resistance_before_vulnerability_and_consumes_temp_hp() {
        let r = calculate_damage(DamageQuery {
            hp: 10,
            max_hp: 20,
            temp_hp: 3,
            amount: 5,
            healing: false,
            resistance: true,
            vulnerability: true,
            immune: false,
        })
        .unwrap();
        assert_eq!(r.adjusted_amount, 4);
        assert_eq!(r.absorbed, 3);
        assert_eq!(r.hp_after, 9);
        let r = calculate_damage(DamageQuery {
            hp: 10,
            max_hp: 20,
            temp_hp: 3,
            amount: 50,
            healing: true,
            resistance: true,
            vulnerability: false,
            immune: true,
        })
        .unwrap();
        assert_eq!(r.hp_after, 20);
        assert_eq!(r.temp_hp_after, 3);
    }
    #[test]
    fn clocks_persist_across_reopen_and_reject_stale_or_foreign_edits() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("tools-test-{}", Uuid::new_v4()));
        let store = CampaignStore::new(path.clone()).unwrap();
        let a = store.create_campaign("Clocks").unwrap().value;
        let b = store.create_campaign("Other clocks").unwrap().value;
        let result = store
            .change_game_tools(
                &a.id,
                ToolMutation {
                    expected_revision: 0,
                    operation: TimerOperation::Add {
                        label: "Torch".into(),
                        clock: "game".into(),
                        mode: "countdown".into(),
                        seconds: 3600,
                    },
                },
            )
            .unwrap()
            .value;
        let id = result.state.timers[0].id.clone();
        assert!(store
            .change_game_tools(
                &a.id,
                ToolMutation {
                    expected_revision: 0,
                    operation: TimerOperation::Start { id: id.clone() }
                }
            )
            .is_err());
        assert!(store
            .change_game_tools(
                &b.id,
                ToolMutation {
                    expected_revision: 0,
                    operation: TimerOperation::Start { id: id.clone() }
                }
            )
            .is_err());
        store
            .change_game_tools(
                &a.id,
                ToolMutation {
                    expected_revision: 1,
                    operation: TimerOperation::Start { id },
                },
            )
            .unwrap();
        store
            .change_game_tools(
                &a.id,
                ToolMutation {
                    expected_revision: 2,
                    operation: TimerOperation::Advance { seconds: 600 },
                },
            )
            .unwrap();
        drop(store);
        let store = CampaignStore::new(path.clone()).unwrap();
        let saved = store.load_game_tools(&a.id).unwrap();
        assert_eq!(saved.revision, 3);
        assert_eq!(saved.state.game_time_ms, 600000);
        assert_eq!(saved.state.timers[0].value_ms, 3000000);
        assert!(store
            .load_game_tools(&b.id)
            .unwrap()
            .state
            .timers
            .is_empty());
        let conn = connect(&store.locate(&a.id).unwrap(), false).unwrap();
        conn.execute_batch("CREATE TRIGGER fail_tools BEFORE INSERT ON events WHEN NEW.event_type='tools.changed' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        assert!(store
            .change_game_tools(
                &a.id,
                ToolMutation {
                    expected_revision: 3,
                    operation: TimerOperation::Advance { seconds: 60 }
                }
            )
            .is_err());
        assert_eq!(store.load_game_tools(&a.id).unwrap().revision, 3);
        drop(conn);
        drop(store);
        assert!(path.starts_with(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target")));
        let _ = fs::remove_dir_all(path);
    }
}
