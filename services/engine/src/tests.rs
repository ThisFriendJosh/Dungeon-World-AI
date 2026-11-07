use super::*;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        Self(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join(format!("dwa-engine-test-{}", Uuid::new_v4())),
        )
    }
    fn store(&self) -> CampaignStore {
        CampaignStore::new(self.0.clone()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if self
            .0
            .starts_with(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"))
            && self
                .0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("dwa-engine-test-")
        {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
fn update() -> CharacterUpdate {
    CharacterUpdate {
        name: "Orrek Bearskin".into(), class_name: "Barbarian".into(), level: 3,
        stats_json: r#"{"str":16,"dex":14,"con":15,"int":8,"wis":12,"cha":10,"hp":19,"maxHp":32,"armorClass":14,"custom":"preserved"}"#.into(),
        inventory_json: r#"["Greataxe","Bedroll"]"#.into(),
        conditions_json: r#"["Poisoned"]"#.into(),
        progression_json: r#"{"notes":"Recover the Sunshard","customFlag":true}"#.into(),
    }
}

#[test]
fn duplicate_names_never_replace_existing_campaigns_or_characters() {
    let f = Fixture::new();
    let store = f.store();
    let first = store.create_campaign("Aerathis").unwrap().value;
    let character = store
        .create_character(&first.id, "Orrek", "Barbarian")
        .unwrap()
        .value;
    let second = store.create_campaign("Aerathis!").unwrap().value;
    assert_ne!(first.id, second.id);
    assert_ne!(first.slug, second.slug);
    assert_eq!(store.load_campaign(&first.slug).unwrap().value.id, first.id);
    assert_eq!(
        store.load_characters(&first.id).unwrap()[0].id,
        character.id
    );
    assert!(store.load_characters(&second.id).unwrap().is_empty());
    assert_eq!(store.list_campaigns().unwrap().len(), 2);
}

#[test]
fn a_saved_sheet_survives_reopening_and_records_the_full_change() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("The Sunshard").unwrap().value;
    let character = store
        .create_character(&campaign.id, "Orrek", "Barbarian")
        .unwrap()
        .value;
    let saved = store
        .update_character(&campaign.id, &character.id, update())
        .unwrap();
    assert!(saved.warning.is_none(), "{:?}", saved.warning);
    drop(store);
    let reopened = f.store();
    let characters = reopened.load_characters(&campaign.id).unwrap();
    assert_eq!(characters.len(), 1);
    assert_eq!(characters[0].name, "Orrek Bearskin");
    assert_eq!(characters[0].level, 3);
    assert_eq!(characters[0].stats_json, update().stats_json);
    assert_eq!(characters[0].inventory_json, update().inventory_json);
    assert_eq!(characters[0].conditions_json, update().conditions_json);
    assert_eq!(characters[0].progression_json, update().progression_json);
    let events = reopened.recent_events(&campaign.id).unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].event_type, "character.updated");
    assert_eq!(events[0].payload["character"]["level"], 3);
    let log = fs::read_to_string(f.0.join(campaign.slug).join("events.log")).unwrap();
    let lines: Vec<Value> = log
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0]["seq"].as_i64() < lines[2]["seq"].as_i64());
}

#[test]
fn missing_and_traversal_slugs_do_not_create_databases() {
    let f = Fixture::new();
    let store = f.store();
    for slug in [
        "missing",
        "../outside",
        "..",
        r"..\outside",
        "/absolute",
        "C:drive",
    ] {
        assert!(store.load_campaign(slug).is_err(), "{slug}");
    }
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 0);
    assert!(store.create_campaign("   ").is_err());
}

#[test]
fn rejects_invalid_sheet_data_without_events_or_partial_updates() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Validation").unwrap().value;
    let character = store
        .create_character(&campaign.id, "Original", "Fighter")
        .unwrap()
        .value;
    let mut bad = update();
    bad.stats_json = r#"{"hp":33,"maxHp":32}"#.into();
    assert!(store
        .update_character(&campaign.id, &character.id, bad)
        .is_err());
    let mut bad = update();
    bad.inventory_json = "{}".into();
    assert!(store
        .update_character(&campaign.id, &character.id, bad)
        .is_err());
    let mut bad = update();
    bad.level = 21;
    assert!(store
        .update_character(&campaign.id, &character.id, bad)
        .is_err());
    assert_eq!(
        store.load_characters(&campaign.id).unwrap()[0].name,
        "Original"
    );
    assert_eq!(store.recent_events(&campaign.id).unwrap().len(), 2);
}

#[test]
fn characters_cannot_be_updated_through_a_different_campaign() {
    let f = Fixture::new();
    let store = f.store();
    let first = store.create_campaign("First").unwrap().value;
    let second = store.create_campaign("Second").unwrap().value;
    let character = store
        .create_character(&first.id, "Original", "Fighter")
        .unwrap()
        .value;
    assert!(store
        .update_character(&second.id, &character.id, update())
        .is_err());
    assert_eq!(
        store.load_characters(&first.id).unwrap()[0].name,
        "Original"
    );
    assert_eq!(store.recent_events(&second.id).unwrap().len(), 1);
}

#[test]
fn mirror_failure_reports_success_and_reopening_repairs_it() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Log recovery").unwrap().value;
    let log = f.0.join(&campaign.slug).join("events.log");
    fs::remove_file(&log).unwrap();
    fs::create_dir(&log).unwrap(); // Deterministically blocks replacement on Windows and Linux.
    let saved = store
        .create_character(&campaign.id, "Still saved", "Wizard")
        .unwrap();
    assert!(saved.warning.is_some());
    assert_eq!(store.load_characters(&campaign.id).unwrap().len(), 1);
    fs::remove_dir(&log).unwrap();
    assert!(store
        .load_campaign(&campaign.slug)
        .unwrap()
        .warning
        .is_none());
    assert_eq!(fs::read_to_string(log).unwrap().lines().count(), 2);
}

#[test]
fn campaign_and_character_changes_roll_back_if_event_insert_fails() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Atomic saves").unwrap().value;
    let character = store
        .create_character(&campaign.id, "Original", "Fighter")
        .unwrap()
        .value;
    let conn = connect(&f.0.join(&campaign.slug), false).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_event BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'event write failed'); END;").unwrap();
    assert!(store
        .update_character(&campaign.id, &character.id, update())
        .is_err());
    assert_eq!(
        store.load_characters(&campaign.id).unwrap()[0].name,
        "Original"
    );
    assert!(store
        .create_character(&campaign.id, "No partial character", "Wizard")
        .is_err());
    assert_eq!(store.load_characters(&campaign.id).unwrap().len(), 1);
}

fn test_scene() -> FariScene {
    let make_card = |title: &str| SceneCard {
        id: Uuid::new_v4().to_string(),
        title_label: "NPC".into(),
        title: title.into(),
        color: "#d6b973".into(),
        played_during_turn: false,
        pinned: false,
        blocks: vec![SceneTextBlock {
            id: Uuid::new_v4().to_string(),
            block_type: "Text".into(),
            label: "Notes".into(),
            value: "Watches the ruined gate.".into(),
            meta: Default::default(),
        }],
        sub_cards: vec![],
        sub: false,
    };
    FariScene {
        id: Uuid::new_v4().to_string(),
        name: "The ruined gate".into(),
        group: None,
        index_cards: SceneCards {
            public: vec![make_card("Gatekeeper")],
            private: vec![make_card("Hidden clue")],
        },
        version: 3,
        last_updated: 1_794_064_878,
        notes: "Recover the Sunshard.".into(),
    }
}

#[test]
fn scene_cards_and_groups_survive_reopening_without_changing_characters() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Scene round trip").unwrap().value;
    let character = store
        .create_character(&campaign.id, "Orrek", "Barbarian")
        .unwrap()
        .value;
    assert!(store.load_tabletop(&campaign.id).unwrap().scene.is_none());
    let saved = store.save_tabletop(&campaign.id, test_scene(), 0).unwrap();
    assert!(saved.warning.is_none(), "{:?}", saved.warning);
    assert_eq!(saved.value.revision, 1);
    drop(store);
    let reopened = f.store();
    let state = reopened.load_tabletop(&campaign.id).unwrap();
    let scene = state.scene.unwrap();
    assert_eq!(scene.name, "The ruined gate");
    assert_eq!(scene.index_cards.public[0].title, "Gatekeeper");
    assert_eq!(scene.index_cards.private[0].title, "Hidden clue");
    assert_eq!(scene.notes, "Recover the Sunshard.");
    assert_eq!(
        reopened.load_characters(&campaign.id).unwrap()[0].id,
        character.id
    );
    let events = reopened.recent_events(&campaign.id).unwrap();
    assert_eq!(events[0].event_type, "scene.saved");
    assert_eq!(
        events[0].payload["scene"]["indexCards"]["private"][0]["title"],
        "Hidden clue"
    );
}

#[test]
fn adding_tabletop_storage_to_a_v0_database_preserves_its_data() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Legacy save").unwrap().value;
    let character = store
        .create_character(&campaign.id, "Original hero", "Fighter")
        .unwrap()
        .value;
    let conn = Connection::open(f.0.join(&campaign.slug).join("campaign.sqlite")).unwrap();
    conn.execute_batch("DROP TABLE tabletops;").unwrap();
    drop(conn);
    drop(store);
    let reopened = f.store();
    assert_eq!(reopened.load_tabletop(&campaign.id).unwrap().revision, 0);
    assert_eq!(
        reopened.load_characters(&campaign.id).unwrap()[0].id,
        character.id
    );
    assert_eq!(reopened.recent_events(&campaign.id).unwrap().len(), 2);
}

#[test]
fn stale_scene_saves_are_rejected_without_overwriting_the_latest_revision() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Revision test").unwrap().value;
    store.save_tabletop(&campaign.id, test_scene(), 0).unwrap();
    let mut latest = test_scene();
    latest.name = "Latest version".into();
    store.save_tabletop(&campaign.id, latest, 1).unwrap();
    let error = store
        .save_tabletop(&campaign.id, test_scene(), 1)
        .unwrap_err();
    assert!(error.to_string().contains("another window"));
    let loaded = store.load_tabletop(&campaign.id).unwrap();
    assert_eq!(loaded.revision, 2);
    assert_eq!(loaded.scene.unwrap().name, "Latest version");
    assert_eq!(store.recent_events(&campaign.id).unwrap().len(), 3);
}

#[test]
fn invalid_scene_content_never_creates_a_scene_or_event() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Scene validation").unwrap().value;
    let mut invalid = test_scene();
    invalid.version = 2;
    assert!(store.save_tabletop(&campaign.id, invalid, 0).is_err());
    let mut invalid = test_scene();
    invalid.index_cards.private[0].id = invalid.index_cards.public[0].id.clone();
    assert!(store.save_tabletop(&campaign.id, invalid, 0).is_err());
    let mut invalid = test_scene();
    invalid.index_cards.public[0].color = "red".into();
    assert!(store.save_tabletop(&campaign.id, invalid, 0).is_err());
    let mut invalid = test_scene();
    invalid.index_cards.public[0].blocks[0].block_type = "Image".into();
    assert!(store.save_tabletop(&campaign.id, invalid, 0).is_err());
    let mut invalid = test_scene();
    invalid.index_cards.public[0].title = " ".into();
    assert!(store.save_tabletop(&campaign.id, invalid, 0).is_err());
    assert_eq!(store.load_tabletop(&campaign.id).unwrap().revision, 0);
    assert_eq!(store.recent_events(&campaign.id).unwrap().len(), 1);
}

#[test]
fn scene_and_event_writes_roll_back_together() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Scene atomic save").unwrap().value;
    store.save_tabletop(&campaign.id, test_scene(), 0).unwrap();
    let conn = connect(&f.0.join(&campaign.slug), false).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_event BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'event failed'); END;").unwrap();
    let mut changed = test_scene();
    changed.name = "Must roll back".into();
    assert!(store.save_tabletop(&campaign.id, changed, 1).is_err());
    let loaded = store.load_tabletop(&campaign.id).unwrap();
    assert_eq!(loaded.revision, 1);
    assert_eq!(loaded.scene.unwrap().name, "The ruined gate");
    assert_eq!(store.recent_events(&campaign.id).unwrap().len(), 2);
}

#[test]
fn separate_campaigns_keep_separate_scenes() {
    let f = Fixture::new();
    let store = f.store();
    let a = store.create_campaign("First scene").unwrap().value;
    let b = store.create_campaign("Second scene").unwrap().value;
    store.save_tabletop(&a.id, test_scene(), 0).unwrap();
    assert!(store.load_tabletop(&b.id).unwrap().scene.is_none());
    assert!(store.load_tabletop("missing-campaign").is_err());
    assert!(store
        .save_tabletop("missing-campaign", test_scene(), 0)
        .is_err());
}
