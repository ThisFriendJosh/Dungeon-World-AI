use super::*;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        Self(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join(format!("dm-test-{}", Uuid::new_v4())),
        )
    }
    fn store(&self) -> CampaignStore {
        CampaignStore::new(self.0.clone()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
        if self.0.starts_with(&root)
            && self
                .0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("dm-test-")
        {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
fn request(store: &CampaignStore, id: &str) -> DmTurnRequest {
    DmTurnRequest {
        request_id: Uuid::new_v4().to_string(),
        actor_id: None,
        player_text: "I ask the gatekeeper about the missing caravan.".into(),
        expected_seq: store.load_dm_history(id).unwrap().latest_event_seq,
    }
}
fn proposal() -> DmProposal {
    DmProposal {
        schema_version: 1,
        narration: "The gatekeeper lowers his lantern. ‘Which caravan?’ he asks.".into(),
        choices: vec!["I describe the missing caravan.".into()],
    }
}

#[test]
fn conversation_is_atomic_idempotent_isolated_and_survives_reopening() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Aerathis").unwrap().value;
    let other = store.create_campaign("Another world").unwrap().value;
    let req = request(&store, &campaign.id);
    let saved = store
        .commit_dm_turn(&campaign.id, &req, proposal(), "test-model", "dm-test-v1")
        .unwrap();
    assert!(saved.warning.is_none());
    let retry = store
        .commit_dm_turn(&campaign.id, &req, proposal(), "test-model", "dm-test-v1")
        .unwrap();
    assert_eq!(saved.value.id, retry.value.id);
    assert_eq!(
        store
            .recent_events(&campaign.id)
            .unwrap()
            .iter()
            .filter(|e| e.event_type == "dm.turn.saved")
            .count(),
        1
    );
    drop(store);
    let reopened = f.store();
    let h = reopened.load_dm_history(&campaign.id).unwrap();
    assert_eq!(h.total_turns, 1);
    assert_eq!(h.turns[0].reply.narration, proposal().narration);
    assert_eq!(h.latest_event_seq, h.turns[0].event_seq);
    assert!(reopened
        .load_dm_history(&other.id)
        .unwrap()
        .turns
        .is_empty());
    let log = fs::read_to_string(f.0.join(&campaign.slug).join("events.log")).unwrap();
    assert_eq!(
        log.lines().filter(|s| s.contains("dm.turn.saved")).count(),
        1
    );
}

#[test]
fn stale_reply_and_cross_campaign_actor_cannot_commit() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("First").unwrap().value;
    let mut req = request(&store, &campaign.id);
    store
        .create_character(&campaign.id, "Orrek", "Barbarian")
        .unwrap();
    assert!(store.prepare_dm_context(&campaign.id, &req).is_err());
    assert!(store
        .commit_dm_turn(&campaign.id, &req, proposal(), "test", "v1")
        .is_err());
    let other = store.create_campaign("Second").unwrap().value;
    let foreign = store
        .create_character(&other.id, "Someone else", "Ranger")
        .unwrap()
        .value;
    req.expected_seq = store
        .load_dm_history(&campaign.id)
        .unwrap()
        .latest_event_seq;
    req.actor_id = Some(foreign.id);
    assert!(store.prepare_dm_context(&campaign.id, &req).is_err());
    assert!(store
        .commit_dm_turn(&campaign.id, &req, proposal(), "test", "v1")
        .is_err());
    assert_eq!(store.load_dm_history(&campaign.id).unwrap().total_turns, 0);
}

#[test]
fn malformed_or_mechanical_proposals_and_request_id_reuse_are_rejected() {
    assert!(serde_json::from_value::<DmProposal>(json!({"schemaVersion":1,"narration":"Gold granted.","choices":[],"proposedEvents":[{"eventType":"inventory.changed"}]})).is_err());
    for p in [
        DmProposal {
            schema_version: 2,
            ..proposal()
        },
        DmProposal {
            narration: " ".into(),
            ..proposal()
        },
        DmProposal {
            narration: "<think>secret</think>".into(),
            ..proposal()
        },
        DmProposal {
            choices: vec!["x".into(); 5],
            ..proposal()
        },
    ] {
        assert!(p.validate().is_err());
    }
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Identity").unwrap().value;
    let mut req = request(&store, &campaign.id);
    store
        .commit_dm_turn(&campaign.id, &req, proposal(), "test", "v1")
        .unwrap();
    req.player_text = "A different action".into();
    assert!(store.find_dm_turn(&campaign.id, &req).is_err());
    assert!(store
        .commit_dm_turn(&campaign.id, &req, proposal(), "test", "v1")
        .is_err());
}

#[test]
fn failed_database_insert_rolls_back_turn_and_event_together() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Rollback").unwrap().value;
    let req = request(&store, &campaign.id);
    let conn = connect(&store.locate(&campaign.id).unwrap(), false).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_dm BEFORE INSERT ON dm_turns BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    assert!(store
        .commit_dm_turn(&campaign.id, &req, proposal(), "test", "v1")
        .is_err());
    let h = store.load_dm_history(&campaign.id).unwrap();
    assert_eq!(h.total_turns, 0);
    assert_eq!(h.latest_event_seq, req.expected_seq);
    assert!(!store
        .recent_events(&campaign.id)
        .unwrap()
        .iter()
        .any(|e| e.event_type == "dm.turn.saved"));
}

#[test]
fn dm_context_separates_gm_secrets_and_includes_only_engine_recorded_dice() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Secrets").unwrap().value;
    let make_card = |text: &str| SceneCard {
        id: Uuid::new_v4().to_string(),
        title_label: "Name".into(),
        title: "Gate".into(),
        color: "#abcdef".into(),
        played_during_turn: false,
        pinned: false,
        blocks: vec![SceneTextBlock {
            id: Uuid::new_v4().to_string(),
            block_type: "Text".into(),
            label: "Notes".into(),
            value: text.into(),
            meta: serde_json::Map::new(),
        }],
        sub_cards: vec![],
        sub: false,
    };
    let scene = FariScene {
        id: Uuid::new_v4().to_string(),
        name: "Old gate".into(),
        group: None,
        index_cards: SceneCards {
            public: vec![make_card("A lantern burns.")],
            private: vec![make_card("SECRET_TRAITOR")],
        },
        version: 3,
        last_updated: 1,
        notes: "SECRET_GM_NOTES".into(),
        board: None,
    };
    store.save_tabletop(&campaign.id, scene, 0).unwrap();
    let roll = store
        .roll_dice(
            &campaign.id,
            DiceRequest {
                expression: "1d20+3".into(),
                mode: DiceMode::Normal,
                character_id: None,
                label: "Persuasion".into(),
            },
        )
        .unwrap()
        .value;
    let req = request(&store, &campaign.id);
    let ctx = store.prepare_dm_context(&campaign.id, &req).unwrap();
    let json = serde_json::to_string(&ctx).unwrap();
    assert!(json.contains("A lantern burns."));
    assert!(!ctx.state["scene"].to_string().contains("SECRET_"));
    assert!(ctx.state["gmContext"]
        .to_string()
        .contains("SECRET_TRAITOR"));
    assert!(ctx.state["gmContext"]["gmKnowledgeOnly"].as_bool().unwrap());
    assert_eq!(ctx.state["recordedDiceRolls"][0]["id"], roll.id);
    assert_eq!(ctx.state["recordedDiceRolls"][0]["total"], roll.total);
    assert_eq!(ctx.state_seq, req.expected_seq);
}

#[test]
fn event_mirror_failure_reports_committed_turn_without_duplication() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Mirror").unwrap().value;
    let mirror = f.0.join(&campaign.slug).join("events.log");
    fs::remove_file(&mirror).unwrap();
    fs::create_dir(&mirror).unwrap();
    let req = request(&store, &campaign.id);
    let result = store
        .commit_dm_turn(&campaign.id, &req, proposal(), "test", "v1")
        .unwrap();
    assert!(result.warning.is_some());
    assert_eq!(store.load_dm_history(&campaign.id).unwrap().total_turns, 1);
    fs::remove_dir(&mirror).unwrap();
    store.load_campaign(&campaign.slug).unwrap();
    assert!(mirror.is_file());
}
