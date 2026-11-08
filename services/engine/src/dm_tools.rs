use super::*;
use serde_json::json;

/// The model can only request these read/calculation operations. No write,
/// filesystem path, URL, SQL statement, or executable expression is accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DmReadRequest {
    Search {
        query: String,
    },
    Reference {
        id: String,
    },
    Scene {
        scene_id: Option<String>,
        token_id: Option<String>,
    },
    Character {
        id: String,
        section: String,
        offset: u32,
    },
    Distance {
        from_id: String,
        to_id: String,
    },
    Arithmetic {
        operation: String,
        a: f64,
        b: f64,
    },
    Inspect {
        scope: String,
        id: Option<String>,
        section: String,
        offset: u32,
    },
}

fn detail_page(value: Value, offset: u32, gm_only: bool) -> Result<Value, EngineError> {
    let encoded = value.to_string();
    let length = encoded.chars().count();
    if offset as usize > length {
        return Err(EngineError::Invalid(
            "Choose a detail offset within range.".into(),
        ));
    }
    if length <= 4000 && offset == 0 {
        return Ok(json!({"data":value,"nextOffset":null,"gmKnowledgeOnly":gm_only}));
    }
    Ok(
        json!({"format":"JSON fragment; continue at nextOffset","text":encoded.chars().skip(offset as usize).take(4000).collect::<String>(),"offset":offset,"totalCharacters":length,"nextOffset":(offset as usize+4000<length).then_some(offset+4000),"gmKnowledgeOnly":gm_only}),
    )
}

pub(crate) fn board_snapshot(board: &SceneBoard) -> Value {
    json!({"columns":board.columns,"rows":board.rows,"feetPerCell":board.feet_per_cell,
        "coordinateConvention":"x/y are zero-based top-left grid coordinates; column/row are one-based; token size is in cells",
        "backgroundImageId":board.background,"fogEnabled":board.fog_enabled,"ambientLight":board.ambient,
        "tokens":board.tokens.iter().map(|t|json!({"id":t.id,"name":t.name,"kind":t.kind,"x":t.x,"y":t.y,
            "column":t.x.floor()+1.0,"row":t.y.floor()+1.0,"sizeCells":t.size,"characterId":t.character_id,
            "hidden":t.hidden,"hp":t.hp,"maxHp":t.max_hp,"states":t.states,"spriteId":t.sprite,
            "brightLightFeet":t.bright_light,"dimLightFeet":t.dim_light})).collect::<Vec<_>>(),
        "drawingCount":board.drawings.len(),"fogMarkCount":board.fog.len(),
        "visibilityWarning":"Hidden tokens and fog are GM knowledge. Line of sight, walls, difficult terrain, cover and paths have not been computed."})
}

pub(crate) fn derived_character(character: &CharacterSummary) -> Value {
    let stats: Value = serde_json::from_str(&character.stats_json).unwrap_or(json!({}));
    let mut modifiers = serde_json::Map::new();
    for key in ["str", "dex", "con", "int", "wis", "cha"] {
        if let Some(score) = stats[key].as_i64().filter(|s| (1..=30).contains(s)) {
            modifiers.insert(key.into(), json!((score - 10).div_euclid(2)));
        }
    }
    json!({"abilityModifiers":modifiers,"proficiencyBonusByLevel":2+(character.level-1)/4,
        "armorClassIsTotal":true,"proficiencyAppliedToChecks":false,
        "note":"Only base ability modifiers and level-based proficiency are computed. Do not assume proficiency, expertise, saves, features, attack/damage bonuses, or spellcasting ability from class/level alone."})
}

impl CampaignStore {
    pub fn read_dm_tool(
        &self,
        campaign_id: &str,
        expected_seq: i64,
        request: DmReadRequest,
    ) -> Result<Value, EngineError> {
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let revision = || -> Result<i64, EngineError> {
            Ok(conn.query_row(
                "SELECT COALESCE(MAX(seq),0) FROM events WHERE campaign_id=?1",
                [campaign_id],
                |r| r.get(0),
            )?)
        };
        if revision()? != expected_seq {
            return Err(EngineError::Invalid(
                "Campaign changed during the DM lookup. Refresh and retry.".into(),
            ));
        }
        let result = match request {
            DmReadRequest::Inspect {
                scope,
                id,
                section,
                offset,
            } => {
                if offset > 100_000_000 {
                    return Err(EngineError::Invalid("Detail offset is too large.".into()));
                }
                match scope.as_str() {
                    "reference" => self.read_dm_reference_page(
                        campaign_id,
                        &id.ok_or_else(|| {
                            EngineError::Invalid("Choose a reference ID returned by search.".into())
                        })?,
                        offset,
                    )?,
                    "document" => {
                        let id = id.ok_or_else(|| {
                            EngineError::Invalid(
                                "Choose a source ID from the document catalog.".into(),
                            )
                        })?;
                        let doc = self.read_knowledge_document(campaign_id, &id, offset)?;
                        json!({"source":doc.source,"original":doc.original,"passage":doc.passages.first(),"offsetUnit":"passage","nextOffset":(offset+1<doc.source.chunk_count as u32).then_some(offset+1),"imageUnderstanding":"Indexed text is available. The narrator does not interpret the original illustrations."})
                    }
                    "catalog" => {
                        let entries=match section.as_str(){"documents"=>serde_json::to_value(self.list_knowledge_sources(campaign_id)?),"scenes"=>serde_json::to_value(self.list_tabletop_scenes(campaign_id)?),"characters"=>Ok(json!(self.load_characters(campaign_id)?.iter().map(|c|json!({"id":c.id,"name":c.name,"class":c.class_name,"level":c.level})).collect::<Vec<_>>())),"art"=>serde_json::to_value(self.list_art(campaign_id)?),"images"=>serde_json::to_value(self.list_scene_images(campaign_id)?),_=>return Err(EngineError::Invalid("Choose documents, scenes, characters, art, or images.".into()))}.map_err(|e|EngineError::Invalid(e.to_string()))?;
                        let entries = entries.as_array().unwrap();
                        json!({"entries":entries.iter().skip(offset as usize).take(10).collect::<Vec<_>>(),"total":entries.len(),"offset":offset,"offsetUnit":"entry","nextOffset":(offset as usize+10<entries.len()).then_some(offset+10),"gmKnowledgeOnly":true})
                    }
                    "character" => {
                        let character = self
                            .load_characters(campaign_id)?
                            .into_iter()
                            .find(|c| Some(&c.id) == id.as_ref())
                            .ok_or_else(|| {
                                EngineError::Invalid("Character not found in this campaign.".into())
                            })?;
                        let value = match section.as_str() {
                            "stats" => serde_json::from_str(&character.stats_json),
                            "inventory" => serde_json::from_str(&character.inventory_json),
                            "conditions" => serde_json::from_str(&character.conditions_json),
                            "notes" => serde_json::from_str(&character.progression_json),
                            "all" => serde_json::to_value(character),
                            _ => {
                                return Err(EngineError::Invalid(
                                    "Choose stats, inventory, conditions, notes, or all.".into(),
                                ))
                            }
                        }
                        .map_err(|e| EngineError::Invalid(e.to_string()))?;
                        detail_page(value, offset, false)?
                    }
                    "scene" => {
                        let scene = if let Some(id) = id {
                            let raw:String=conn.query_row("SELECT scene_json FROM tabletop_scenes WHERE campaign_id=?1 AND scene_id=?2",params![campaign_id,id],|r|r.get(0)).optional()?.ok_or_else(||EngineError::Invalid("Scene not found in this campaign.".into()))?;
                            serde_json::from_str::<FariScene>(&raw)
                                .map_err(|e| EngineError::Invalid(e.to_string()))?
                        } else {
                            self.load_tabletop(campaign_id)?
                                .scene
                                .ok_or_else(|| EngineError::Invalid("No saved scene.".into()))?
                        };
                        let value = serde_json::to_value(scene)
                            .map_err(|e| EngineError::Invalid(e.to_string()))?;
                        let selected = match section.as_str() {
                            "all" => value,
                            "notes" => value["notes"].clone(),
                            "cards" => value["indexCards"].clone(),
                            "cardCatalog"=>json!(["public","private"].iter().flat_map(|visibility|value["indexCards"][*visibility].as_array().into_iter().flatten().map(move |card|json!({"id":card["id"],"title":card["title"],"gmOnly":*visibility=="private"}))).collect::<Vec<_>>()),
                            _ if section.starts_with("card:")=>["public","private"].iter().flat_map(|visibility|value["indexCards"][*visibility].as_array().into_iter().flatten()).find(|card|card["id"].as_str()==section.strip_prefix("card:")).cloned().ok_or_else(||EngineError::Invalid("Card not found in this scene.".into()))?,
                            _ if section.starts_with("token:")=>value["board"]["tokens"].as_array().into_iter().flatten().find(|token|token["id"].as_str()==section.strip_prefix("token:")).cloned().ok_or_else(||EngineError::Invalid("Token not found in this scene.".into()))?,
                            "board" => value["board"].clone(),
                            "drawings" => value["board"]["drawings"].clone(),
                            "fog" => value["board"]["fog"].clone(),
                            _ => {
                                return Err(EngineError::Invalid(
                                    "Choose all, notes, cards, board, drawings, or fog.".into(),
                                ))
                            }
                        };
                        detail_page(selected, offset, true)?
                    }
                    "progress" => match section.as_str() {
                        "calculations" => {
                            let records = self.list_calculations(campaign_id, offset)?;
                            let next = (records.len() == 20).then_some(offset + 20);
                            json!({"entries":records,"offset":offset,"offsetUnit":"entry","nextOffset":next,"previewOnly":true})
                        }
                        "rolls" => {
                            let rolls = self.load_dice_rolls(campaign_id)?;
                            json!({"entries":rolls.iter().skip(offset as usize).take(10).collect::<Vec<_>>(),"offset":offset,"offsetUnit":"entry","nextOffset":(offset as usize+10<rolls.len()).then_some(offset+10)})
                        }
                        "timers" => detail_page(
                            serde_json::to_value(self.load_game_tools(campaign_id)?)
                                .map_err(|e| EngineError::Invalid(e.to_string()))?,
                            offset,
                            true,
                        )?,
                        "activity" => {
                            if let Some(id) = id {
                                let seq = id.parse::<i64>().map_err(|_| {
                                    EngineError::Invalid(
                                        "Choose an activity sequence from the saved list.".into(),
                                    )
                                })?;
                                let payload:Option<String>=conn.query_row("SELECT payload_json FROM events e WHERE e.campaign_id=?1 AND e.seq=?2 AND NOT EXISTS(SELECT 1 FROM event_visibility v WHERE v.campaign_id=e.campaign_id AND v.event_seq=e.seq AND v.removed=1)",params![campaign_id,seq],|r|r.get(0)).optional()?;
                                detail_page(
                                    serde_json::from_str(&payload.ok_or_else(|| {
                                        EngineError::Invalid(
                                            "Activity not found in this campaign.".into(),
                                        )
                                    })?)
                                    .map_err(|e| EngineError::Invalid(e.to_string()))?,
                                    offset,
                                    true,
                                )?
                            } else {
                                let mut statement=conn.prepare("SELECT seq,event_type,created_at FROM events e WHERE e.campaign_id=?1 AND NOT EXISTS(SELECT 1 FROM event_visibility v WHERE v.campaign_id=e.campaign_id AND v.event_seq=e.seq AND v.removed=1) ORDER BY e.seq DESC LIMIT 11 OFFSET ?2")?;
                                let events=statement.query_map(params![campaign_id,offset],|r|Ok(json!({"id":r.get::<_,i64>(0)?.to_string(),"kind":r.get::<_,String>(1)?,"at":r.get::<_,String>(2)?})))?.collect::<Result<Vec<_>,_>>()?;
                                json!({"entries":events.iter().take(10).collect::<Vec<_>>(),"offsetUnit":"entry","nextOffset":(events.len()>10).then_some(offset+10),"gmKnowledgeOnly":true,"hint":"Use an entry id with activity to read its full saved payload."})
                            }
                        }
                        _ => {
                            return Err(EngineError::Invalid(
                                "Choose calculations, rolls, timers, or activity.".into(),
                            ))
                        }
                    },
                    _ => {
                        return Err(EngineError::Invalid(
                            "Choose document, catalog, character, scene, or progress inspection."
                                .into(),
                        ))
                    }
                }
            }
            DmReadRequest::Search { query } => {
                json!({"matches":self.search_dm_references(campaign_id,&query)?,"hint":"Use reference with a returned id for a larger passage. Edition labels and GM-only flags are authoritative metadata."})
            }
            DmReadRequest::Reference { id } => self.read_dm_reference(campaign_id, &id)?,
            DmReadRequest::Character {
                id,
                section,
                offset,
            } => {
                let characters = self.load_characters(campaign_id)?;
                let c = characters
                    .iter()
                    .find(|c| c.id == id)
                    .ok_or_else(|| EngineError::CharacterNotFound(id.clone()))?;
                if offset > 100_000 {
                    return Err(EngineError::Invalid(
                        "Invalid character page offset.".into(),
                    ));
                }
                let raw = match section.as_str() {
                    "stats" => &c.stats_json,
                    "inventory" => &c.inventory_json,
                    "conditions" => &c.conditions_json,
                    "notes" => &c.progression_json,
                    "summary" => "",
                    _ => {
                        return Err(EngineError::Invalid(
                            "Choose summary, stats, inventory, conditions, or notes.".into(),
                        ))
                    }
                };
                if section == "summary" {
                    json!({"id":c.id,"name":c.name,"class":c.class_name,"level":c.level,"derived":derived_character(c)})
                } else {
                    let value: Value = serde_json::from_str(raw)
                        .map_err(|e| EngineError::Invalid(e.to_string()))?;
                    let entries: Vec<Value> = match value {
                        Value::Array(items) => items,
                        Value::Object(map) => map
                            .into_iter()
                            .map(|(key, value)| json!({"field":key,"value":value}))
                            .collect(),
                        value => vec![value],
                    };
                    let selected = entries
                        .iter()
                        .skip(offset as usize)
                        .take(10)
                        .map(|v| {
                            let encoded = v.to_string();
                            if encoded.chars().count() > 3000 {
                                json!({"text":knowledge::clip(&encoded,3000),"truncated":true})
                            } else {
                                v.clone()
                            }
                        })
                        .collect::<Vec<_>>();
                    json!({"characterId":id,"section":section,"offset":offset,"totalEntries":entries.len(),"entries":selected,"nextOffset":if offset as usize+10<entries.len(){Some(offset+10)}else{None},"gmKnowledgeOnly":false})
                }
            }
            DmReadRequest::Scene { scene_id, token_id } => {
                let scene = if let Some(id) = scene_id {
                    let raw:String=conn.query_row("SELECT scene_json FROM tabletop_scenes WHERE campaign_id=?1 AND scene_id=?2",params![campaign_id,id],|r|r.get(0)).optional()?.ok_or_else(||EngineError::Invalid("Scene not found in this campaign.".into()))?;
                    serde_json::from_str::<FariScene>(&raw)
                        .map_err(|e| EngineError::Invalid(e.to_string()))?
                } else {
                    self.load_tabletop(campaign_id)?.scene.ok_or_else(|| {
                        EngineError::Invalid("No active tabletop scene is saved.".into())
                    })?
                };
                if let Some(token_id) = token_id {
                    let token = scene
                        .board
                        .as_ref()
                        .and_then(|b| b.tokens.iter().find(|t| t.id == token_id))
                        .ok_or_else(|| {
                            EngineError::Invalid("Token not found in this scene.".into())
                        })?;
                    let mut value = serde_json::to_value(token)
                        .map_err(|e| EngineError::Invalid(e.to_string()))?;
                    value["notes"] = json!(knowledge::clip(&token.notes, 5000));
                    value["notesTruncated"] = json!(token.notes.chars().count() > 5000);
                    json!({"gmKnowledgeOnly":true,"token":value})
                } else {
                    let cards = |cards: &[SceneCard]| {
                        cards.iter().map(|c|json!({"id":c.id,"title":c.title,"text":c.blocks.first().map(|b|knowledge::clip(&b.value,800)).unwrap_or_default()})).collect::<Vec<_>>()
                    };
                    json!({"gmKnowledgeOnly":true,"id":scene.id,"name":scene.name,"notes":knowledge::clip(&scene.notes,3000),
                        "publicCards":cards(&scene.index_cards.public),"privateCards":cards(&scene.index_cards.private),
                        "board":scene.board.as_ref().map(board_snapshot)})
                }
            }
            DmReadRequest::Distance { from_id, to_id } => {
                let scene = self
                    .load_tabletop(campaign_id)?
                    .scene
                    .ok_or_else(|| EngineError::Invalid("No active scene.".into()))?;
                let board = scene
                    .board
                    .ok_or_else(|| EngineError::Invalid("No saved grid in this scene.".into()))?;
                let find = |id: &str| {
                    board.tokens.iter().find(|t| t.id == id).ok_or_else(|| {
                        EngineError::Invalid("Choose tokens on the active saved grid.".into())
                    })
                };
                let a = find(&from_id)?;
                let b = find(&to_id)?;
                let dx = (a.x + f64::from(a.size) / 2.0) - (b.x + f64::from(b.size) / 2.0);
                let dy = (a.y + f64::from(a.size) / 2.0) - (b.y + f64::from(b.size) / 2.0);
                let feet = f64::from(board.feet_per_cell);
                json!({"fromId":a.id,"toId":b.id,"centerDistanceGridFeet":dx.abs().max(dy.abs())*feet,"centerDistanceStraightFeet":dx.hypot(dy)*feet,
                    "method":"Center-to-center. Grid metric counts each diagonal as one cell. No movement cost, path, line-of-sight, cover, reach or targeting ruling has been made.","gmKnowledgeOnly":a.hidden||b.hidden||board.fog_enabled})
            }
            DmReadRequest::Arithmetic { operation, a, b } => {
                if !a.is_finite() || !b.is_finite() || a.abs() > 1e9 || b.abs() > 1e9 {
                    return Err(EngineError::Invalid(
                        "Use finite arithmetic inputs up to one billion.".into(),
                    ));
                }
                let result=match operation.as_str(){"add"=>a+b,"subtract"=>a-b,"multiply"=>a*b,"divide" if b!=0.0=>a/b,"floor_divide" if b!=0.0=>(a/b).floor(),_=>return Err(EngineError::Invalid("Choose add, subtract, multiply, divide or floor_divide; division by zero is unavailable.".into()))};
                json!({"a":a,"b":b,"operation":operation,"result":result,"source":"engine calculation"})
            }
        };
        if revision()? != expected_seq {
            return Err(EngineError::Invalid(
                "Campaign changed during the DM lookup. Refresh and retry.".into(),
            ));
        }
        if result.to_string().len() > 80_000 {
            return Err(EngineError::Invalid("This lookup is too large. Request a narrower character section or a single token/reference.".into()));
        }
        Ok(result)
    }
}
