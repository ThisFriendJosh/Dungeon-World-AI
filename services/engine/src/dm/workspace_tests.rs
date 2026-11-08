use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
struct Data(PathBuf);
impl Data {
    fn new() -> Self {
        Self(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join(format!("dm-workspace-{}", Uuid::new_v4())),
        )
    }
    fn store(&self) -> CampaignStore {
        CampaignStore::new(self.0.clone()).unwrap()
    }
}
impl Drop for Data {
    fn drop(&mut self) {
        if self
            .0
            .starts_with(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"))
            && self
                .0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("dm-workspace-")
        {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
fn seq(store: &CampaignStore, id: &str) -> i64 {
    store.load_dm_history(id).unwrap().latest_event_seq
}
fn source(title: &str, text: &str, edition: &str) -> KnowledgeImport {
    KnowledgeImport {
        title: title.into(),
        text: text.into(),
        source_ref: "Original manuscript, version 2".into(),
        edition: edition.into(),
        gm_only: true,
    }
}
fn edit(
    store: &CampaignStore,
    id: &str,
    event_seq: i64,
    removed: bool,
) -> Result<Saved<Value>, EngineError> {
    store.edit_history_entry(
        id,
        HistoryEdit {
            event_seq,
            expected_seq: seq(store, id),
            removed,
            confirmed: true,
        },
    )
}

#[test]
fn campaign_knowledge_is_searchable_after_reopen_with_source_and_gm_boundaries() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("Aerathis").unwrap().value;
    let other = store.create_campaign("Other").unwrap().value;
    store.import_knowledge(&campaign.id,source("Sunshard Road","Mara Fen saw the Sunshard stolen at Mournwell. SECRET_GM_RELIC is beneath the chapel.","homebrew")).unwrap();
    drop(store);
    let store = data.store();
    let hits = store
        .search_dm_references(&campaign.id, "Mournwell")
        .unwrap();
    let hit = hits.iter().find(|h| h.kind == "campaign").unwrap();
    assert!(hit.gm_only);
    assert_eq!(hit.source_ref, "Original manuscript, version 2");
    assert!(
        store.read_dm_reference(&campaign.id, &hit.id).unwrap()["text"]
            .as_str()
            .unwrap()
            .contains("SECRET_GM_RELIC")
    );
    assert!(store
        .search_dm_references(&other.id, "Mournwell")
        .unwrap()
        .iter()
        .all(|h| h.kind != "campaign"));
    assert!(store.read_dm_reference(&other.id, &hit.id).is_err());
    assert!(store
        .search_dm_references(&campaign.id, "\" OR * NOT - '")
        .unwrap()
        .is_empty());
}

#[test]
fn reference_search_covers_descriptions_and_respects_edition_labels() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("Editions").unwrap().value;
    store
        .import_content(
            &campaign.id,
            ContentImport {
                name: "Spells".into(),
                kind: "spells".into(),
                entries: vec![
                    ContentEntry {
                        name: "Veil".into(),
                        description: "Moonember reveals a hidden trail.".into(),
                        properties: json!({}),
                        publisher: "Test".into(),
                        book: "Test 2024".into(),
                    },
                    ContentEntry {
                        name: "Old Veil".into(),
                        description: "Moonember old rules.".into(),
                        properties: json!({}),
                        publisher: "Test".into(),
                        book: "Test 2014".into(),
                    },
                ],
            },
        )
        .unwrap();
    store
        .import_knowledge(
            &campaign.id,
            source(
                "Old edition",
                "Moonember THAC0 descending armor class.",
                "AD&D-2e",
            ),
        )
        .unwrap();
    let hits = store
        .search_dm_references(&campaign.id, "Moonember")
        .unwrap();
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().any(|h| h.title == "Veil"));
    assert!(hits.iter().any(|h| h.title == "Old edition"));
    let sources = store.list_knowledge_sources(&campaign.id).unwrap();
    assert_eq!(sources[0].edition, "AD&D-2e");
    let conn = connect(&store.locate(&campaign.id).unwrap(), false).unwrap();
    let foreign_id: String = conn
        .query_row(
            "SELECT reference_id FROM dm_reference_index WHERE edition='AD&D-2e'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        store.read_dm_reference(&campaign.id, &foreign_id).unwrap()["edition"],
        "AD&D-2e"
    );
    assert!(store
        .search_dm_references(&campaign.id, "advantage disadvantage")
        .unwrap()
        .iter()
        .any(|h| h.id.starts_with("srd:5.2.1:")));
}

#[test]
fn reference_index_backfills_existing_compendium_once_and_import_failure_is_atomic() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("Index migration").unwrap().value;
    store
        .import_content(
            &campaign.id,
            ContentImport {
                name: "Items".into(),
                kind: "items".into(),
                entries: vec![ContentEntry {
                    name: "Moonember stone".into(),
                    description: "A test stone".into(),
                    properties: json!({}),
                    publisher: "".into(),
                    book: "".into(),
                }],
            },
        )
        .unwrap();
    let conn = connect(&store.locate(&campaign.id).unwrap(), false).unwrap();
    conn.execute_batch(
        "DELETE FROM dm_reference_index;DELETE FROM engine_metadata WHERE key='dm-reference-v1';",
    )
    .unwrap();
    drop(conn);
    assert_eq!(
        store
            .search_dm_references(&campaign.id, "Moonember")
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        store
            .search_dm_references(&campaign.id, "Moonember")
            .unwrap()
            .len(),
        1
    );
    let conn = connect(&store.locate(&campaign.id).unwrap(), false).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_knowledge BEFORE INSERT ON knowledge_chunks BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    assert!(store
        .import_knowledge(
            &campaign.id,
            source("Rollback", "A Moonember secret.", "homebrew")
        )
        .is_err());
    assert!(store
        .list_knowledge_sources(&campaign.id)
        .unwrap()
        .is_empty());
    assert_eq!(
        store
            .search_dm_references(&campaign.id, "Moonember")
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn removing_and_restoring_a_turn_changes_memory_without_undoing_saved_state() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("History").unwrap().value;
    let character = store
        .create_character(&campaign.id, "Bearskin", "Barbarian")
        .unwrap()
        .value;
    let request = DmTurnRequest {
        request_id: Uuid::new_v4().to_string(),
        actor_id: Some(character.id.clone()),
        player_text: "I enter the wrong scene.".into(),
        expected_seq: seq(&store, &campaign.id),
    };
    let turn = store
        .commit_dm_turn(
            &campaign.id,
            &request,
            DmProposal {
                schema_version: 1,
                narration: "An accidental test scene.".into(),
                choices: vec![],
            },
            "test",
            "v1",
        )
        .unwrap()
        .value;
    let sheet = store.load_characters(&campaign.id).unwrap()[0]
        .stats_json
        .clone();
    edit(&store, &campaign.id, turn.event_seq, true).unwrap();
    assert_eq!(store.load_dm_history(&campaign.id).unwrap().total_turns, 0);
    assert!(!store
        .recent_events(&campaign.id)
        .unwrap()
        .iter()
        .any(|e| e.seq == turn.event_seq));
    assert!(store.find_dm_turn(&campaign.id, &request).is_err());
    assert_eq!(
        store.load_characters(&campaign.id).unwrap()[0].stats_json,
        sheet
    );
    assert!(
        fs::read_to_string(data.0.join(&campaign.slug).join("events.log"))
            .unwrap()
            .contains("dm.turn.saved")
    );
    assert!(
        store
            .load_editable_history(&campaign.id)
            .unwrap()
            .iter()
            .find(|e| e.event.seq == turn.event_seq)
            .unwrap()
            .removed
    );
    edit(&store, &campaign.id, turn.event_seq, false).unwrap();
    assert_eq!(
        store.load_dm_history(&campaign.id).unwrap().turns[0].id,
        turn.id
    );
}

#[test]
fn history_edits_require_confirmation_revision_and_campaign_ownership_and_roll_back() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("First").unwrap().value;
    let other = store.create_campaign("Second").unwrap().value;
    let roll = store
        .roll_dice(
            &campaign.id,
            DiceRequest {
                expression: "1d20".into(),
                mode: DiceMode::Normal,
                character_id: None,
                label: "Test".into(),
            },
        )
        .unwrap()
        .value;
    let event_seq = seq(&store, &campaign.id);
    assert!(store
        .edit_history_entry(
            &campaign.id,
            HistoryEdit {
                event_seq,
                expected_seq: event_seq,
                removed: true,
                confirmed: false
            }
        )
        .is_err());
    assert!(edit(&store, &other.id, event_seq, true).is_err());
    let conn = connect(&store.locate(&campaign.id).unwrap(), false).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_history BEFORE INSERT ON events WHEN NEW.event_type='history.entry.removed' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    assert!(edit(&store, &campaign.id, event_seq, true).is_err());
    assert_eq!(store.load_dice_rolls(&campaign.id).unwrap()[0].id, roll.id);
    conn.execute_batch("DROP TRIGGER fail_history;").unwrap();
    edit(&store, &campaign.id, event_seq, true).unwrap();
    assert!(store.load_dice_rolls(&campaign.id).unwrap().is_empty());
    assert!(store
        .edit_history_entry(
            &campaign.id,
            HistoryEdit {
                event_seq,
                expected_seq: event_seq,
                removed: false,
                confirmed: true
            }
        )
        .is_err());
    assert!(edit(&store, &campaign.id, seq(&store, &campaign.id), true).is_err());
    // audit row
}

fn grid_scene() -> FariScene {
    let token = |name: &str, x: f64, y: f64, hidden: bool| json!({"id":Uuid::new_v4().to_string(),"name":name,"kind":"npc","x":x,"y":y,"size":1,"color":"#abcdef","sprite":null,"characterId":null,"notes":"Secret patrol route","states":[],"hp":12,"maxHp":12,"hidden":hidden,"locked":false,"rotation":0,"brightLight":0,"dimLight":0});
    serde_json::from_value(json!({"id":Uuid::new_v4().to_string(),"name":"Saved grid","indexCards":{"public":[],"private":[]},"version":3,"lastUpdated":1,"notes":"GM knowledge","board":{"columns":30,"rows":20,"feetPerCell":5,"grid":true,"snap":true,"gridOpacity":0.2,"background":null,"tokens":[token("Hero",1.0,1.0,false),token("Hidden guard",4.0,5.0,true)],"drawings":[],"fogEnabled":true,"fog":[],"ambient":0}})).unwrap()
}
#[test]
fn dm_reads_exact_saved_grid_and_calculates_distance_without_inventing_visibility() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("Grid").unwrap().value;
    let scene = grid_scene();
    let tokens = &scene.board.as_ref().unwrap().tokens;
    let a = tokens[0].id.clone();
    let b = tokens[1].id.clone();
    store.save_tabletop(&campaign.id, scene, 0).unwrap();
    let current = seq(&store, &campaign.id);
    let result = store
        .read_dm_tool(
            &campaign.id,
            current,
            DmReadRequest::Distance {
                from_id: a,
                to_id: b.clone(),
            },
        )
        .unwrap();
    assert_eq!(result["centerDistanceGridFeet"], 20.0);
    assert_eq!(result["centerDistanceStraightFeet"], 25.0);
    assert_eq!(result["gmKnowledgeOnly"], true);
    let token = store
        .read_dm_tool(
            &campaign.id,
            current,
            DmReadRequest::Scene {
                scene_id: None,
                token_id: Some(b),
            },
        )
        .unwrap();
    assert_eq!(token["token"]["x"], 4.0);
    assert_eq!(token["token"]["hidden"], true);
    assert!(store
        .read_dm_tool(
            &campaign.id,
            current - 1,
            DmReadRequest::Arithmetic {
                operation: "add".into(),
                a: 1.0,
                b: 2.0
            }
        )
        .is_err());
}

#[test]
fn engine_arithmetic_and_character_sections_do_not_infer_missing_training() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("Arithmetic").unwrap().value;
    let character = store
        .create_character(&campaign.id, "Bearskin", "Barbarian")
        .unwrap()
        .value;
    let current = seq(&store, &campaign.id);
    let result = store
        .read_dm_tool(
            &campaign.id,
            current,
            DmReadRequest::Arithmetic {
                operation: "floor_divide".into(),
                a: -1.0,
                b: 2.0,
            },
        )
        .unwrap();
    assert_eq!(result["result"], -1.0);
    assert!(store
        .read_dm_tool(
            &campaign.id,
            current,
            DmReadRequest::Arithmetic {
                operation: "divide".into(),
                a: 1.0,
                b: 0.0
            }
        )
        .is_err());
    let summary = store
        .read_dm_tool(
            &campaign.id,
            current,
            DmReadRequest::Character {
                id: character.id,
                section: "summary".into(),
                offset: 0,
            },
        )
        .unwrap();
    assert_eq!(summary["derived"]["proficiencyAppliedToChecks"], false);
    assert!(
        serde_json::from_value::<DmReadRequest>(json!({"kind":"shell","command":"anything"}))
            .is_err()
    );
}

fn png_header() -> String {
    let mut data = vec![0u8; 33];
    data[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    data[12..16].copy_from_slice(b"IHDR");
    data[16..20].copy_from_slice(&512u32.to_be_bytes());
    data[20..24].copy_from_slice(&512u32.to_be_bytes());
    format!("data:image/png;base64,{}", STANDARD.encode(data))
}
fn spec() -> TokenSpec {
    TokenSpec {
        name: "<script>Bad name</script>".into(),
        label: "<Hero&>".into(),
        color: "#345678".into(),
        shape: "hex".into(),
        symbol: "shield".into(),
    }
}
#[test]
fn generated_asset_persists_png_svg_and_recipe_atomically_with_xml_escaping() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("Assets").unwrap().value;
    let other = store.create_campaign("Other assets").unwrap().value;
    let asset = store
        .save_token_asset(
            &campaign.id,
            TokenAssetRequest {
                spec: spec(),
                png_data_url: png_header(),
                artwork: None,
            },
        )
        .unwrap()
        .value;
    drop(store);
    let store = data.store();
    let saved = store.load_generated_asset(&campaign.id, &asset.id).unwrap();
    let svg = saved["svg"].as_str().unwrap();
    assert!(!svg.contains("<script>"));
    assert!(svg.contains("&lt;Hero&amp;&gt;"));
    assert_eq!(
        store.list_generated_assets(&campaign.id).unwrap()[0]
            .spec
            .color,
        "#345678"
    );
    assert_eq!(
        store.list_scene_images(&campaign.id).unwrap()[0].id,
        asset.id
    );
    assert!(store.load_generated_asset(&other.id, &asset.id).is_err());
    let conn = connect(&store.locate(&campaign.id).unwrap(), false).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_asset BEFORE INSERT ON generated_assets BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    assert!(store
        .save_token_asset(
            &campaign.id,
            TokenAssetRequest {
                spec: spec(),
                png_data_url: png_header(),
                artwork: None,
            }
        )
        .is_err());
    assert_eq!(store.list_scene_images(&campaign.id).unwrap().len(), 1);
    let mut invalid = spec();
    invalid.color = "url(x)".into();
    assert!(store.preview_token_asset(invalid).is_err());
}

#[test]
fn document_reader_preserves_original_and_reads_every_saved_section_after_reopen() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("Reader").unwrap().value;
    let other = store.create_campaign("Private").unwrap().value;
    let text = format!(
        "{}\nTAIL_SECRET_RELIC",
        "Lore of the ancient gate. ".repeat(800)
    );
    let original = b"original source bytes including tables and illustrations";
    let imported = store
        .import_knowledge_with_original(
            &campaign.id,
            source("Gate manuscript", &text, "5e-2014"),
            Some(OriginalDocumentImport {
                name: "Gate manuscript.txt".into(),
                data_base64: STANDARD.encode(original),
            }),
        )
        .unwrap()
        .value;
    let old = store
        .import_knowledge(
            &campaign.id,
            source("Older text", "Existing text remains readable.", "homebrew"),
        )
        .unwrap()
        .value;
    drop(store);
    let store = data.store();
    let doc = store
        .read_knowledge_document(&campaign.id, &imported.id, 0)
        .unwrap();
    assert!(doc.next_offset.is_some());
    assert_eq!(doc.original.unwrap().byte_count, original.len() as i64);
    let hit = store
        .search_dm_references(&campaign.id, "TAIL_SECRET_RELIC")
        .unwrap()
        .into_iter()
        .find(|h| h.kind == "campaign")
        .unwrap();
    assert_eq!(
        store
            .read_knowledge_document(&campaign.id, &hit.id, 0)
            .unwrap()
            .source
            .id,
        imported.id
    );
    let found = store
        .search_knowledge_document(&campaign.id, &imported.id, "TAIL_SECRET_RELIC")
        .unwrap();
    assert!(found[0].ordinal >= 6);
    let last = store
        .read_knowledge_document(&campaign.id, &imported.id, found[0].ordinal)
        .unwrap();
    assert!(last.passages[0].text.contains("TAIL_SECRET_RELIC"));
    assert!(last.next_offset.is_none());
    assert_eq!(
        store
            .knowledge_original_bytes(&campaign.id, &hit.id)
            .unwrap()
            .1,
        original
    );
    assert!(store
        .read_knowledge_document(&campaign.id, &old.id, 0)
        .unwrap()
        .original
        .is_none());
    assert!(store
        .knowledge_original_bytes(&campaign.id, &old.id)
        .is_err());
    assert!(store
        .read_knowledge_document(&other.id, &hit.id, 0)
        .is_err());
    assert!(store
        .knowledge_original_bytes(&other.id, &imported.id)
        .is_err());
    assert!(store
        .attach_knowledge_original(
            &other.id,
            &old.id,
            OriginalDocumentImport {
                name: "Gate.txt".into(),
                data_base64: STANDARD.encode(original)
            }
        )
        .is_err());
    store
        .attach_knowledge_original(
            &campaign.id,
            &old.id,
            OriginalDocumentImport {
                name: "Gate.txt".into(),
                data_base64: STANDARD.encode(original),
            },
        )
        .unwrap();
    assert_eq!(
        store
            .read_knowledge_document(&campaign.id, &old.id, 0)
            .unwrap()
            .passages[0]
            .text,
        "Existing text remains readable."
    );
}

#[test]
fn original_document_and_index_save_atomically_and_reject_invalid_uploads() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("Atomic documents").unwrap().value;
    let before = seq(&store, &campaign.id);
    for (name, bytes) in [
        ("../escape.txt", "eA=="),
        ("broken.pdf", "eA=="),
        ("empty.txt", ""),
        ("bad.exe", "eA=="),
        ("bad.txt", "%%%"),
    ] {
        assert!(store
            .import_knowledge_with_original(
                &campaign.id,
                source("Bad file", "test text", "homebrew"),
                Some(OriginalDocumentImport {
                    name: name.into(),
                    data_base64: bytes.into()
                })
            )
            .is_err());
    }
    let conn = connect(&store.locate(&campaign.id).unwrap(), false).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_original BEFORE INSERT ON knowledge_originals BEGIN SELECT RAISE(ABORT,'fixture write failure'); END;").unwrap();
    assert!(store
        .import_knowledge_with_original(
            &campaign.id,
            source("Rollback original", "rollbackmarker", "homebrew"),
            Some(OriginalDocumentImport {
                name: "file.txt".into(),
                data_base64: STANDARD.encode("file")
            })
        )
        .is_err());
    assert_eq!(seq(&store, &campaign.id), before);
    assert!(store
        .list_knowledge_sources(&campaign.id)
        .unwrap()
        .is_empty());
    assert!(store
        .search_dm_references(&campaign.id, "rollbackmarker")
        .unwrap()
        .is_empty());
}

fn inspect(
    store: &CampaignStore,
    campaign: &str,
    scope: &str,
    id: Option<String>,
    section: &str,
    offset: u32,
) -> Result<Value, EngineError> {
    store.read_dm_tool(
        campaign,
        seq(store, campaign),
        DmReadRequest::Inspect {
            scope: scope.into(),
            id,
            section: section.into(),
            offset,
        },
    )
}
fn read_detail(
    store: &CampaignStore,
    campaign: &str,
    scope: &str,
    id: Option<String>,
    section: &str,
) -> Value {
    let mut offset = 0;
    let mut encoded = String::new();
    loop {
        let page = inspect(store, campaign, scope, id.clone(), section, offset).unwrap();
        if !page["data"].is_null() {
            return page["data"].clone();
        }
        encoded.push_str(page["text"].as_str().unwrap());
        if let Some(next) = page["nextOffset"].as_u64() {
            assert!(next > offset as u64);
            offset = next as u32;
        } else {
            break;
        }
    }
    serde_json::from_str(&encoded).unwrap()
}

#[test]
fn saved_calculations_are_engine_computed_previews_and_visible_in_dm_context() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("Awareness").unwrap().value;
    let other = store.create_campaign("Other awareness").unwrap().value;
    let actor = store
        .create_character(&campaign.id, "Test hero", "Barbarian")
        .unwrap()
        .value;
    let sheet = store.load_characters(&campaign.id).unwrap()[0]
        .stats_json
        .clone();
    let before = seq(&store, &campaign.id);
    let bad = CalculationRequest {
        character_id: Some(Uuid::new_v4().to_string()),
        operation: CalculationOperation::Arithmetic {
            expression: "1+1".into(),
        },
    };
    assert!(store.calculate_campaign(&campaign.id, bad).is_err());
    assert_eq!(before, seq(&store, &campaign.id));
    let math = store
        .calculate_campaign(
            &campaign.id,
            CalculationRequest {
                character_id: Some(actor.id.clone()),
                operation: CalculationOperation::Arithmetic {
                    expression: "(12+8)/2".into(),
                },
            },
        )
        .unwrap()
        .value;
    assert_eq!(math.result, 10.0);
    assert!(math.preview_only);
    let rules = store
        .calculate_campaign(
            &campaign.id,
            CalculationRequest {
                character_id: Some(actor.id.clone()),
                operation: CalculationOperation::Rules {
                    query: RulesQuery {
                        ability_score: 16,
                        level: 3,
                        damage: 24,
                        exhaustion: 0,
                        speed: 30,
                    },
                },
            },
        )
        .unwrap()
        .value;
    assert_eq!(rules.result["concentrationDc"], 12);
    assert_eq!(rules.result["profile"], "5e-2024");
    let damage=store.calculate_campaign(&campaign.id,serde_json::from_value(json!({"characterId":actor.id,"operation":{"kind":"damage","query":{"hp":10,"maxHp":20,"tempHp":3,"amount":9,"healing":false,"resistance":true,"vulnerability":false,"immune":false}}})).unwrap()).unwrap().value;
    assert_eq!(damage.result["hpAfter"], 9);
    assert_eq!(damage.result["tempHpAfter"], 0);
    let revision = store.load_game_tools(&campaign.id).unwrap().revision;
    store
        .change_game_tools(
            &campaign.id,
            ToolMutation {
                expected_revision: revision,
                operation: TimerOperation::Advance { seconds: 600 },
            },
        )
        .unwrap();
    let roll = store
        .roll_dice(
            &campaign.id,
            DiceRequest {
                expression: "1d20+3".into(),
                mode: DiceMode::Normal,
                character_id: Some(actor.id.clone()),
                label: "Guard persuasion".into(),
            },
        )
        .unwrap()
        .value;
    store.save_tabletop(&campaign.id, grid_scene(), 0).unwrap();
    let request = DmTurnRequest {
        request_id: Uuid::new_v4().to_string(),
        actor_id: Some(actor.id.clone()),
        player_text: "What has happened?".into(),
        expected_seq: seq(&store, &campaign.id),
    };
    let context = store.prepare_dm_context(&campaign.id, &request).unwrap();
    assert_eq!(context.state["timekeeping"]["gameTimeMs"], 600000);
    assert_eq!(
        context.state["calculationHistory"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(context.state["recordedDiceRolls"][0]["id"], roll.id);
    assert_eq!(context.state["gmContext"]["board"]["tokens"][1]["x"], 4.0);
    assert_eq!(context.state["party"][0]["id"], actor.id);
    assert_eq!(
        store.load_characters(&campaign.id).unwrap()[0].stats_json,
        sheet
    );
    assert!(store.list_calculations(&other.id, 0).unwrap().is_empty());
    assert!(serde_json::from_value::<CalculationRequest>(json!({"characterId":null,"operation":{"kind":"arithmetic","expression":"1+1"},"result":999})).is_err());
    drop(store);
    let store = data.store();
    assert_eq!(store.list_calculations(&campaign.id, 0).unwrap().len(), 3);
    edit(&store, &campaign.id, math.event_seq, true).unwrap();
    assert_eq!(store.list_calculations(&campaign.id, 0).unwrap().len(), 2);
    assert!(inspect(
        &store,
        &campaign.id,
        "progress",
        Some(math.event_seq.to_string()),
        "activity",
        0
    )
    .is_err());
    let list = inspect(&store, &campaign.id, "progress", None, "activity", 0).unwrap();
    assert!(!list["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["id"] == math.event_seq.to_string()));
}

#[test]
fn dm_can_inspect_full_scene_blocks_character_notes_and_document_tails() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("Full details").unwrap().value;
    let other = store.create_campaign("Other details").unwrap().value;
    let mut scene = grid_scene();
    scene.notes = format!("{}TAIL_SCENE_NOTE", "n".repeat(8000));
    let card:SceneCard=serde_json::from_value(json!({"id":Uuid::new_v4().to_string(),"titleLabel":"NPC","title":"Gatekeeper","color":"#abcdef","playedDuringTurn":false,"pinned":false,"blocks":[{"id":Uuid::new_v4().to_string(),"type":"Text","label":"Visible","value":format!("{}TAIL_CARD_DETAIL","c".repeat(8000)),"meta":{}}],"subCards":[],"sub":false})).unwrap();
    scene.index_cards.private.push(card);
    let scene_id = scene.id.clone();
    store.save_tabletop(&campaign.id, scene, 0).unwrap();
    let cards = read_detail(&store, &campaign.id, "scene", None, "cards");
    assert!(cards["private"][0]["blocks"][0]["value"]
        .as_str()
        .unwrap()
        .ends_with("TAIL_CARD_DETAIL"));
    let card_id = cards["private"][0]["id"].as_str().unwrap();
    let card = read_detail(
        &store,
        &campaign.id,
        "scene",
        None,
        &format!("card:{card_id}"),
    );
    assert!(card["blocks"][0]["value"]
        .as_str()
        .unwrap()
        .ends_with("TAIL_CARD_DETAIL"));
    let notes = read_detail(&store, &campaign.id, "scene", None, "notes");
    assert!(notes.as_str().unwrap().ends_with("TAIL_SCENE_NOTE"));
    assert!(inspect(&store, &other.id, "scene", Some(scene_id), "all", 0).is_err());
    let actor = store
        .create_character(&campaign.id, "Test hero", "Barbarian")
        .unwrap()
        .value;
    let conn = connect(&store.locate(&campaign.id).unwrap(), false).unwrap();
    let long_notes = json!({"journal":format!("{}TAIL_CHARACTER_JOURNAL","n".repeat(11000))});
    conn.execute(
        "UPDATE characters SET progression_json=?1 WHERE id=?2",
        params![long_notes.to_string(), actor.id],
    )
    .unwrap();
    assert_eq!(
        read_detail(
            &store,
            &campaign.id,
            "character",
            Some(actor.id.clone()),
            "notes"
        ),
        long_notes
    );
    assert!(inspect(&store, &other.id, "character", Some(actor.id), "notes", 0).is_err());
    let source = store
        .import_knowledge(
            &campaign.id,
            source(
                "Long reference",
                &format!("{}TAIL_DOCUMENT", "Lore ".repeat(3000)),
                "homebrew",
            ),
        )
        .unwrap()
        .value;
    let mut offset = 0;
    let mut text = String::new();
    loop {
        let page = inspect(
            &store,
            &campaign.id,
            "document",
            Some(source.id.clone()),
            "",
            offset,
        )
        .unwrap();
        text.push_str(page["passage"]["text"].as_str().unwrap());
        if let Some(next) = page["nextOffset"].as_u64() {
            offset = next as u32;
        } else {
            break;
        }
    }
    assert!(text.ends_with("TAIL_DOCUMENT"));
    assert!(inspect(&store, &other.id, "document", Some(source.id), "", 0).is_err());
}

#[test]
fn calculation_and_activity_pages_reach_older_results_without_hidden_entries() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("Long progress").unwrap().value;
    for n in 0..23 {
        store
            .calculate_campaign(
                &campaign.id,
                CalculationRequest {
                    character_id: None,
                    operation: CalculationOperation::Arithmetic {
                        expression: format!("{n}+1"),
                    },
                },
            )
            .unwrap();
    }
    let first = inspect(&store, &campaign.id, "progress", None, "calculations", 0).unwrap();
    assert_eq!(first["entries"].as_array().unwrap().len(), 20);
    assert_eq!(first["nextOffset"], 20);
    let last = inspect(&store, &campaign.id, "progress", None, "calculations", 20).unwrap();
    assert_eq!(last["entries"].as_array().unwrap().len(), 3);
    assert_eq!(last["entries"][2]["result"], 1.0);
    let activity = inspect(&store, &campaign.id, "progress", None, "activity", 0).unwrap();
    assert_eq!(activity["nextOffset"], 10);
    let older = inspect(&store, &campaign.id, "progress", None, "activity", 10).unwrap();
    assert_ne!(older["entries"][0]["id"], activity["entries"][0]["id"]);
    let payload = read_detail(
        &store,
        &campaign.id,
        "progress",
        Some(older["entries"][0]["id"].as_str().unwrap().into()),
        "activity",
    );
    assert_eq!(payload["previewOnly"], true);
}

#[test]
fn full_reference_paging_preserves_long_fields_and_keeps_rules_edition_boundaries() {
    let data = Data::new();
    let store = data.store();
    let campaign = store.create_campaign("Reference pages").unwrap().value;
    store
        .import_content(
            &campaign.id,
            ContentImport {
                name: "Lore items".into(),
                kind: "items".into(),
                entries: vec![
                    ContentEntry {
                        name: "Chronicle stone".into(),
                        description: format!("{}TAIL_REFERENCE_FIELD", "lore ".repeat(2400)),
                        properties: json!({}),
                        publisher: "Test".into(),
                        book: "Test 2024".into(),
                    },
                    ContentEntry {
                        name: "Legacy stone".into(),
                        description: "Editionlocked rules.".into(),
                        properties: json!({}),
                        publisher: "Test".into(),
                        book: "Test 2014".into(),
                    },
                ],
            },
        )
        .unwrap();
    let hit = store
        .search_dm_references(&campaign.id, "TAIL_REFERENCE_FIELD")
        .unwrap()
        .into_iter()
        .find(|h| h.title == "Chronicle stone")
        .unwrap();
    let mut offset = 0;
    let mut text = String::new();
    loop {
        let page = inspect(
            &store,
            &campaign.id,
            "reference",
            Some(hit.id.clone()),
            "text",
            offset,
        )
        .unwrap();
        text.push_str(page["text"].as_str().unwrap());
        if let Some(next) = page["nextOffset"].as_u64() {
            offset = next as u32;
        } else {
            break;
        }
    }
    assert!(text.contains("TAIL_REFERENCE_FIELD"));
    let conn = connect(&store.locate(&campaign.id).unwrap(), false).unwrap();
    let legacy: String = conn
        .query_row(
            "SELECT reference_id FROM dm_reference_index WHERE title='Legacy stone'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(inspect(&store, &campaign.id, "reference", Some(legacy), "text", 0).is_err());
}
