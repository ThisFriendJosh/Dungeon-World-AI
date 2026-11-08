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
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Veil");
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
    assert!(store.read_dm_reference(&campaign.id, &foreign_id).is_err());
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
                png_data_url: png_header()
            }
        )
        .is_err());
    assert_eq!(store.list_scene_images(&campaign.id).unwrap().len(), 1);
    let mut invalid = spec();
    invalid.color = "url(x)".into();
    assert!(store.preview_token_asset(invalid).is_err());
}
