use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RulesProfile {
    #[serde(rename = "5e-manual")]
    Manual,
    #[serde(rename = "5e-2014")]
    Legacy2014,
    #[serde(rename = "5e-2024")]
    Revised2024,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HouseRules {
    pub flanking_advantage: bool,
    pub potion_as_bonus_action: bool,
    pub crit_max_one_die: bool,
}
impl Default for HouseRules {
    fn default() -> Self {
        Self {
            flanking_advantage: false,
            potion_as_bonus_action: true,
            crit_max_one_die: false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RulesConfig {
    pub schema_version: u32,
    pub profile: RulesProfile,
    pub house_rules: HouseRules,
}
impl RulesConfig {
    pub(crate) fn modern() -> Self {
        Self {
            schema_version: 1,
            profile: RulesProfile::Revised2024,
            house_rules: HouseRules::default(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesState {
    pub config: RulesConfig,
    pub revision: i64,
    pub updated_at: String,
    pub source_version: Option<String>,
}
fn source_version(profile: RulesProfile) -> Option<String> {
    match profile {
        RulesProfile::Manual => None,
        RulesProfile::Legacy2014 => Some("SRD-5.1".into()),
        RulesProfile::Revised2024 => Some("SRD-5.2.1".into()),
    }
}
pub(crate) fn initialize_new_rules(
    tx: &Transaction<'_>,
    campaign_id: &str,
    timestamp: &str,
) -> Result<(), EngineError> {
    let config = serde_json::to_string(&RulesConfig::modern())
        .map_err(|e| EngineError::Invalid(e.to_string()))?;
    tx.execute("INSERT INTO campaign_rules(campaign_id,config_json,revision,updated_at) VALUES (?1,?2,1,?3)",params![campaign_id,config,timestamp])?;
    Ok(())
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RulesQuery {
    pub ability_score: i64,
    pub level: i64,
    pub damage: i64,
    pub exhaustion: u32,
    pub speed: i64,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesCalculation {
    pub profile: RulesProfile,
    pub ability_modifier: i64,
    pub proficiency_bonus: i64,
    pub trained_bonus: i64,
    pub spell_save_dc: i64,
    pub concentration_dc: Option<i64>,
    pub exhaustion_test_penalty: Option<i64>,
    pub speed_after_exhaustion: Option<i64>,
    pub exhaustion_state: String,
}
pub(crate) fn calculate(
    profile: RulesProfile,
    query: RulesQuery,
) -> Result<RulesCalculation, EngineError> {
    if !(1..=30).contains(&query.ability_score)
        || !(1..=20).contains(&query.level)
        || !(0..=10000).contains(&query.damage)
        || query.exhaustion > 6
        || !(0..=1000).contains(&query.speed)
    {
        return Err(EngineError::Invalid("Use an ability score 1-30, level 1-20, damage 0-10000, exhaustion 0-6, and speed 0-1000.".into()));
    }
    let ability_modifier = (query.ability_score - 10).div_euclid(2);
    let proficiency_bonus = 2 + (query.level - 1) / 4;
    let concentration_dc = if query.damage == 0 || profile == RulesProfile::Manual {
        None
    } else {
        let dc = 10.max(query.damage / 2);
        Some(if profile == RulesProfile::Revised2024 {
            dc.min(30)
        } else {
            dc
        })
    };
    let revised = profile == RulesProfile::Revised2024;
    let dead = revised && query.exhaustion == 6;
    Ok(RulesCalculation {
        profile,
        ability_modifier,
        proficiency_bonus,
        trained_bonus: ability_modifier + proficiency_bonus,
        spell_save_dc: 8 + ability_modifier + proficiency_bonus,
        concentration_dc,
        exhaustion_test_penalty: if revised && !dead {
            Some(-2 * i64::from(query.exhaustion))
        } else {
            None
        },
        speed_after_exhaustion: if revised {
            Some(if dead {
                0
            } else {
                (query.speed - 5 * i64::from(query.exhaustion)).max(0)
            })
        } else {
            None
        },
        exhaustion_state: if !revised {
            "manual"
        } else if dead {
            "dead"
        } else {
            "active"
        }
        .into(),
    })
}
impl CampaignStore {
    pub fn load_rules_settings(&self, campaign_id: &str) -> Result<RulesState, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let (json, revision, updated_at): (String, i64, String) = conn.query_row(
            "SELECT config_json,revision,updated_at FROM campaign_rules WHERE campaign_id=?1",
            [campaign_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let config: RulesConfig = serde_json::from_str(&json)
            .map_err(|e| EngineError::Invalid(format!("Saved rules settings are invalid: {e}")))?;
        if config.schema_version != 1 {
            return Err(EngineError::Invalid(
                "Unsupported rules-settings version.".into(),
            ));
        }
        Ok(RulesState {
            source_version: source_version(config.profile),
            config,
            revision,
            updated_at,
        })
    }
    pub fn save_rules_settings(
        &self,
        campaign_id: &str,
        config: RulesConfig,
        expected_revision: i64,
    ) -> Result<Saved<RulesState>, EngineError> {
        if config.schema_version != 1 || expected_revision < 1 {
            return Err(EngineError::Invalid(
                "Invalid rules settings or revision.".into(),
            ));
        }
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let tx = conn.transaction()?;
        let current: i64 = tx.query_row(
            "SELECT revision FROM campaign_rules WHERE campaign_id=?1",
            [campaign_id],
            |r| r.get(0),
        )?;
        if current != expected_revision {
            return Err(EngineError::Invalid("Rules settings changed in another window. Refresh before saving; your selection is still on screen.".into()));
        }
        let revision = current
            .checked_add(1)
            .ok_or_else(|| EngineError::Invalid("Rules revision limit reached.".into()))?;
        let timestamp = now();
        let json =
            serde_json::to_string(&config).map_err(|e| EngineError::Invalid(e.to_string()))?;
        tx.execute("UPDATE campaign_rules SET config_json=?1,revision=?2,updated_at=?3 WHERE campaign_id=?4",params![json,revision,timestamp,campaign_id])?;
        record_event(
            &tx,
            campaign_id,
            "rules.changed",
            &serde_json::json!({"config":config,"revision":revision,"sourceVersion":source_version(config.profile)}),
            &timestamp,
        )?;
        tx.commit()?;
        Ok(saved(
            RulesState {
                source_version: source_version(config.profile),
                config,
                revision,
                updated_at: timestamp,
            },
            &dir,
            &conn,
            campaign_id,
        ))
    }
    pub fn calculate_rules(
        &self,
        campaign_id: &str,
        query: RulesQuery,
    ) -> Result<RulesCalculation, EngineError> {
        calculate(self.load_rules_settings(campaign_id)?.config.profile, query)
    }
}
