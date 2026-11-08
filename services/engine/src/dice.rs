use super::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiceMode {
    #[default]
    Normal,
    Advantage,
    Disadvantage,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiceRequest {
    pub expression: String,
    #[serde(default)]
    pub mode: DiceMode,
    #[serde(default)]
    pub character_id: Option<String>,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum DiceTerm {
    Dice {
        sign: i64,
        count: u32,
        sides: u32,
        rolls: Vec<u32>,
        kept: Vec<bool>,
        subtotal: i64,
    },
    Modifier {
        value: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiceRoll {
    pub id: String,
    pub campaign_id: String,
    pub character_id: Option<String>,
    pub actor_name: Option<String>,
    pub label: String,
    pub expression: String,
    pub mode: DiceMode,
    pub terms: Vec<DiceTerm>,
    pub total: i64,
    pub created_at: String,
}

#[derive(Debug, Clone)]
enum TermSpec {
    Dice { sign: i64, count: u32, sides: u32 },
    Modifier(i64),
}

fn invalid_formula() -> EngineError {
    EngineError::Invalid("Use dice notation such as 1d20+5 or 2d6+1d4-2.".into())
}

fn parse_formula(input: &str) -> Result<Vec<TermSpec>, EngineError> {
    if input.is_empty() || input.len() > 120 || !input.is_ascii() {
        return Err(invalid_formula());
    }
    let compact: String = input
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let bytes = compact.as_bytes();
    let mut pos = 0;
    let mut specs = Vec::new();
    let mut dice_count = 0u32;
    while pos < bytes.len() {
        let sign = match bytes[pos] {
            b'+' => {
                pos += 1;
                1
            }
            b'-' => {
                pos += 1;
                -1
            }
            _ if specs.is_empty() => 1,
            _ => return Err(invalid_formula()),
        };
        let start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
        let digits = &compact[start..pos];
        if pos < bytes.len() && bytes[pos] == b'd' {
            let count = if digits.is_empty() {
                1
            } else {
                digits.parse::<u32>().map_err(|_| invalid_formula())?
            };
            pos += 1;
            let sides_start = pos;
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            let sides = compact[sides_start..pos]
                .parse::<u32>()
                .map_err(|_| invalid_formula())?;
            if !(1..=100).contains(&count) || !(2..=1000).contains(&sides) {
                return Err(EngineError::Invalid(
                    "Use 1-100 dice per term, with 2-1000 sides.".into(),
                ));
            }
            dice_count += count;
            if dice_count > 100 {
                return Err(EngineError::Invalid(
                    "A roll can contain up to 100 dice.".into(),
                ));
            }
            specs.push(TermSpec::Dice { sign, count, sides });
        } else {
            let value = digits.parse::<i64>().map_err(|_| invalid_formula())?;
            if value > 10_000 {
                return Err(EngineError::Invalid(
                    "Each modifier must be at most 10000.".into(),
                ));
            }
            specs.push(TermSpec::Modifier(sign * value));
        }
        if specs.len() > 16 {
            return Err(EngineError::Invalid(
                "Use at most 16 terms in a formula.".into(),
            ));
        }
    }
    if dice_count == 0 {
        return Err(EngineError::Invalid("Include at least one die.".into()));
    }
    Ok(specs)
}

fn normalized_formula(specs: &[TermSpec]) -> String {
    let mut result = String::new();
    for (index, spec) in specs.iter().enumerate() {
        let (sign, text) = match spec {
            TermSpec::Dice { sign, count, sides } => (*sign, format!("{count}d{sides}")),
            TermSpec::Modifier(value) => (if *value < 0 { -1 } else { 1 }, value.abs().to_string()),
        };
        if sign < 0 {
            result.push('-');
        } else if index > 0 {
            result.push('+');
        }
        result.push_str(&text);
    }
    result
}

// Rejection sampling keeps every die face equally likely without modulo bias.
fn sample_die(
    sides: u32,
    random: &mut impl FnMut() -> Result<u32, EngineError>,
) -> Result<u32, EngineError> {
    let threshold = sides.wrapping_neg() % sides;
    loop {
        let value = random()?;
        if value >= threshold {
            return Ok(value % sides + 1);
        }
    }
}

fn evaluate(
    specs: &[TermSpec],
    mode: DiceMode,
    random: &mut impl FnMut() -> Result<u32, EngineError>,
) -> Result<(Vec<DiceTerm>, i64), EngineError> {
    if mode != DiceMode::Normal {
        let d20: Vec<_> = specs
            .iter()
            .filter(|s| matches!(s, TermSpec::Dice { sides: 20, .. }))
            .collect();
        if d20.len() != 1
            || !matches!(
                d20[0],
                TermSpec::Dice {
                    sign: 1,
                    count: 1,
                    ..
                }
            )
        {
            return Err(EngineError::Invalid(
                "Advantage/disadvantage requires exactly one positive 1d20 term.".into(),
            ));
        }
        let dice: u32 = specs
            .iter()
            .map(|s| match s {
                TermSpec::Dice { count, .. } => *count,
                _ => 0,
            })
            .sum();
        if dice >= 100 {
            return Err(EngineError::Invalid(
                "A roll can contain up to 100 physical dice, including advantage.".into(),
            ));
        }
    }
    let mut terms = Vec::new();
    let mut total = 0;
    for spec in specs {
        match spec {
            TermSpec::Modifier(value) => {
                total += value;
                terms.push(DiceTerm::Modifier { value: *value });
            }
            TermSpec::Dice { sign, count, sides } => {
                let use_two = mode != DiceMode::Normal && *sides == 20;
                let number = if use_two { 2 } else { *count };
                let mut rolls = Vec::new();
                for _ in 0..number {
                    rolls.push(sample_die(*sides, random)?);
                }
                let mut kept = vec![true; number as usize];
                if use_two {
                    let first = if mode == DiceMode::Advantage {
                        rolls[0] >= rolls[1]
                    } else {
                        rolls[0] <= rolls[1]
                    };
                    kept = vec![first, !first];
                }
                let subtotal: i64 = rolls
                    .iter()
                    .zip(&kept)
                    .filter(|(_, k)| **k)
                    .map(|(v, _)| i64::from(*v) * sign)
                    .sum();
                total += subtotal;
                terms.push(DiceTerm::Dice {
                    sign: *sign,
                    count: *count,
                    sides: *sides,
                    rolls,
                    kept,
                    subtotal,
                });
            }
        }
    }
    Ok((terms, total))
}

impl CampaignStore {
    pub fn roll_dice(
        &self,
        campaign_id: &str,
        request: DiceRequest,
    ) -> Result<Saved<DiceRoll>, EngineError> {
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let specs = parse_formula(&request.expression)?;
        let label = request.label.trim().to_string();
        if label.chars().count() > 120 {
            return Err(EngineError::Invalid(
                "Roll label must be at most 120 characters.".into(),
            ));
        }
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let actor_name = if let Some(id) = &request.character_id {
            Some(
                conn.query_row(
                    "SELECT name FROM characters WHERE id = ?1 AND campaign_id = ?2",
                    params![id, campaign_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .ok_or_else(|| EngineError::CharacterNotFound(id.clone()))?,
            )
        } else {
            None
        };
        let mut random = || {
            let mut bytes = [0u8; 4];
            getrandom::fill(&mut bytes).map_err(|e| {
                EngineError::Invalid(format!("System randomness is unavailable: {e}"))
            })?;
            Ok(u32::from_le_bytes(bytes))
        };
        let (terms, total) = evaluate(&specs, request.mode, &mut random)?;
        let roll = DiceRoll {
            id: Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            character_id: request.character_id,
            actor_name,
            label,
            expression: normalized_formula(&specs),
            mode: request.mode,
            terms,
            total,
            created_at: now(),
        };
        let tx = conn.transaction()?;
        let payload =
            serde_json::to_value(&roll).map_err(|e| EngineError::Invalid(e.to_string()))?;
        record_event(&tx, campaign_id, "dice.rolled", &payload, &roll.created_at)?;
        tx.commit()?;
        Ok(saved(roll, &dir, &conn, campaign_id))
    }

    pub fn load_dice_rolls(&self, campaign_id: &str) -> Result<Vec<DiceRoll>, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let mut stmt = conn.prepare("SELECT payload_json FROM events e WHERE campaign_id = ?1 AND event_type = 'dice.rolled' AND NOT EXISTS(SELECT 1 FROM event_visibility v WHERE v.campaign_id=e.campaign_id AND v.event_seq=e.seq AND v.removed=1) ORDER BY seq DESC LIMIT 50")?;
        let rows = stmt.query_map([campaign_id], |row| row.get::<_, String>(0))?;
        let mut rolls = Vec::new();
        for row in rows {
            let roll: DiceRoll = serde_json::from_str(&row?).map_err(|e| {
                EngineError::Invalid(format!("Saved dice history could not be read: {e}"))
            })?;
            if roll.campaign_id != campaign_id {
                return Err(EngineError::Invalid(
                    "Dice history contains a mismatched campaign.".into(),
                ));
            }
            rolls.push(roll);
        }
        Ok(rolls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    fn evaluate_values(input: &str, mode: DiceMode, values: Vec<u32>) -> (Vec<DiceTerm>, i64) {
        let specs = parse_formula(input).unwrap();
        let mut values = VecDeque::from(values);
        evaluate(&specs, mode, &mut || {
            Ok(values.pop_front().expect("unexpected random draw"))
        })
        .unwrap()
    }

    #[test]
    fn formula_supports_multiple_signed_terms_and_whitespace() {
        let specs = parse_formula(" 2D6 + d4 - 2 ").unwrap();
        assert_eq!(normalized_formula(&specs), "2d6+1d4-2");
        let (_, total) = evaluate_values("2d6+d4-2", DiceMode::Normal, vec![6, 11, 3]);
        assert_eq!(total, 9); // d6: 1 and 6; d4: 4; modifier: -2.
    }
    #[test]
    fn subtraction_applies_to_each_kept_die() {
        let (_, total) = evaluate_values("d20-d4+2", DiceMode::Normal, vec![29, 3]);
        assert_eq!(total, 8); // 10 - 4 + 2.
    }
    #[test]
    fn advantage_keeps_only_the_higher_d20() {
        let (terms, total) = evaluate_values("d20+5", DiceMode::Advantage, vec![23, 36]);
        assert_eq!(total, 22);
        assert!(
            matches!(&terms[0], DiceTerm::Dice { rolls, kept, .. } if rolls == &[4,17] && kept == &[false,true])
        );
    }
    #[test]
    fn disadvantage_keeps_only_the_lower_d20() {
        let (_, total) = evaluate_values("d20+5", DiceMode::Disadvantage, vec![23, 36]);
        assert_eq!(total, 9);
    }
    #[test]
    fn a_tie_keeps_one_die_and_bless_rolls_once() {
        let (terms, total) = evaluate_values("d20+d4+3", DiceMode::Advantage, vec![29, 29, 2]);
        assert_eq!(total, 16);
        assert!(matches!(&terms[0], DiceTerm::Dice { kept, .. } if kept == &[true,false]));
        assert!(matches!(&terms[1], DiceTerm::Dice { rolls, .. } if rolls.len() == 1));
    }
    #[test]
    fn invalid_formulas_and_limits_are_rejected() {
        for input in [
            "",
            "5",
            "0d20",
            "101d6",
            "1d1",
            "d1001",
            "d20+",
            "d20++2",
            "2**3",
            "d20;alert(1)",
            "d20+10001",
            "99d6+2d6",
            "d20+１",
        ] {
            assert!(parse_formula(input).is_err(), "{input}");
        }
        assert!(parse_formula(&"1".repeat(121)).is_err());
        assert!(parse_formula("d6+d6+d6+d6+d6+d6+d6+d6+d6+d6+d6+d6+d6+d6+d6+d6+d6").is_err());
    }
    #[test]
    fn advantage_rejects_ambiguous_or_non_d20_rolls_before_randomness() {
        for input in ["2d20+5", "d6+3", "d20+d20", "-d20+4", "d20+99d6"] {
            let specs = parse_formula(input).unwrap();
            assert!(evaluate(&specs, DiceMode::Advantage, &mut || panic!(
                "must validate before rolling"
            ))
            .is_err());
        }
    }
    #[test]
    fn rejection_sampling_skips_values_that_would_bias_a_d6() {
        let mut values = VecDeque::from(vec![0, 1, 2, 3, 4]);
        assert_eq!(
            sample_die(6, &mut || Ok(values.pop_front().unwrap())).unwrap(),
            5
        );
        assert!(values.is_empty());
    }
    #[test]
    fn random_source_failure_is_reported() {
        let specs = parse_formula("d20").unwrap();
        assert!(
            evaluate(&specs, DiceMode::Normal, &mut || Err(EngineError::Invalid(
                "random failed".into()
            )))
            .is_err()
        );
    }
}
