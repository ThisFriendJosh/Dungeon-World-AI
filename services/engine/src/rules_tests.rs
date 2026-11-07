use super::*;
fn query(score: i64, level: i64, damage: i64, exhaustion: u32) -> RulesQuery {
    RulesQuery {
        ability_score: score,
        level,
        damage,
        exhaustion,
        speed: 30,
    }
}
#[test]
fn revised_calculations_round_negative_modifiers_and_follow_proficiency_tiers() {
    for (score, expected) in [
        (1, -5),
        (7, -2),
        (8, -1),
        (9, -1),
        (10, 0),
        (16, 3),
        (30, 10),
    ] {
        let v = rules::calculate(RulesProfile::Revised2024, query(score, 1, 0, 0)).unwrap();
        assert_eq!(v.ability_modifier, expected);
        assert_eq!(v.trained_bonus, expected + 2);
        assert_eq!(v.spell_save_dc, 10 + expected);
    }
    for (level, pb) in [
        (1, 2),
        (4, 2),
        (5, 3),
        (8, 3),
        (9, 4),
        (12, 4),
        (13, 5),
        (16, 5),
        (17, 6),
        (20, 6),
    ] {
        assert_eq!(
            rules::calculate(RulesProfile::Revised2024, query(10, level, 0, 0))
                .unwrap()
                .proficiency_bonus,
            pb
        );
    }
}
#[test]
fn concentration_uses_profile_specific_caps_and_skips_zero_damage() {
    for damage in [1, 19, 20, 21, 39, 60, 99, 10000] {
        let revised = rules::calculate(RulesProfile::Revised2024, query(10, 1, damage, 0)).unwrap();
        let legacy = rules::calculate(RulesProfile::Legacy2014, query(10, 1, damage, 0)).unwrap();
        assert_eq!(revised.concentration_dc, Some(10.max(damage / 2).min(30)));
        assert_eq!(legacy.concentration_dc, Some(10.max(damage / 2)));
    }
    assert_eq!(
        rules::calculate(RulesProfile::Revised2024, query(16, 1, 0, 0))
            .unwrap()
            .concentration_dc,
        None
    );
    assert_eq!(
        rules::calculate(RulesProfile::Manual, query(16, 1, 100, 0))
            .unwrap()
            .concentration_dc,
        None
    );
}
#[test]
fn exhaustion_does_not_mix_legacy_and_revised_rules() {
    let revised = rules::calculate(RulesProfile::Revised2024, query(16, 1, 0, 3)).unwrap();
    assert_eq!(revised.exhaustion_test_penalty, Some(-6));
    assert_eq!(revised.speed_after_exhaustion, Some(15));
    assert_eq!(revised.exhaustion_state, "active");
    let mut slow = query(10, 1, 0, 5);
    slow.speed = 10;
    assert_eq!(
        rules::calculate(RulesProfile::Revised2024, slow)
            .unwrap()
            .speed_after_exhaustion,
        Some(0)
    );
    let dead = rules::calculate(RulesProfile::Revised2024, query(16, 1, 0, 6)).unwrap();
    assert_eq!(dead.exhaustion_state, "dead");
    assert_eq!(dead.exhaustion_test_penalty, None);
    for profile in [RulesProfile::Manual, RulesProfile::Legacy2014] {
        let v = rules::calculate(profile, query(16, 1, 0, 3)).unwrap();
        assert_eq!(v.exhaustion_test_penalty, None);
        assert_eq!(v.speed_after_exhaustion, None);
        assert_eq!(v.exhaustion_state, "manual");
    }
}
#[test]
fn invalid_rules_queries_are_rejected() {
    for bad in [
        query(0, 1, 0, 0),
        query(31, 1, 0, 0),
        query(10, 0, 0, 0),
        query(10, 21, 0, 0),
        query(10, 1, -1, 0),
        query(10, 1, 10001, 0),
        query(10, 1, 0, 7),
    ] {
        assert!(rules::calculate(RulesProfile::Revised2024, bad).is_err());
    }
}
#[test]
fn new_campaigns_use_revised_rules_and_legacy_databases_remain_manual() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Rules edition").unwrap().value;
    assert_eq!(
        store
            .load_rules_settings(&campaign.id)
            .unwrap()
            .config
            .profile,
        RulesProfile::Revised2024
    );
    let character = store
        .create_character(&campaign.id, "Bearskin", "Barbarian")
        .unwrap()
        .value;
    store
        .update_character(&campaign.id, &character.id, update())
        .unwrap();
    let before = serde_json::to_value(store.load_characters(&campaign.id).unwrap()).unwrap();
    let conn = connect(&f.0.join(&campaign.slug), false).unwrap();
    conn.execute_batch("DROP TABLE campaign_rules;").unwrap();
    drop(conn);
    drop(store);
    let reopened = f.store();
    let state = reopened.load_rules_settings(&campaign.id).unwrap();
    assert_eq!(state.config.profile, RulesProfile::Manual);
    assert_eq!(state.config.house_rules, HouseRules::default());
    assert_eq!(state.revision, 1);
    assert_eq!(state.source_version, None);
    assert_eq!(
        serde_json::to_value(reopened.load_characters(&campaign.id).unwrap()).unwrap(),
        before
    );
}
#[test]
fn rule_profiles_persist_independently_without_rewriting_characters_or_scenes() {
    let f = Fixture::new();
    let store = f.store();
    let a = store.create_campaign("Rules A").unwrap().value;
    let b = store.create_campaign("Rules B").unwrap().value;
    let character = store
        .create_character(&a.id, "Bearskin", "Barbarian")
        .unwrap()
        .value;
    store
        .update_character(&a.id, &character.id, update())
        .unwrap();
    let scene = test_scene();
    store.save_tabletop(&a.id, scene.clone(), 0).unwrap();
    let before = serde_json::to_value(store.load_characters(&a.id).unwrap()).unwrap();
    let mut config = store.load_rules_settings(&a.id).unwrap().config;
    config.profile = RulesProfile::Legacy2014;
    config.house_rules.flanking_advantage = true;
    let saved = store.save_rules_settings(&a.id, config.clone(), 1).unwrap();
    assert_eq!(saved.value.revision, 2);
    assert!(saved.warning.is_none());
    assert!(store.save_rules_settings(&a.id, config, 1).is_err());
    let mut bad = RulesConfig::modern();
    bad.schema_version = 99;
    assert!(store.save_rules_settings(&a.id, bad, 2).is_err());
    drop(store);
    let reopened = f.store();
    assert_eq!(
        reopened.load_rules_settings(&a.id).unwrap().config.profile,
        RulesProfile::Legacy2014
    );
    assert_eq!(
        reopened.load_rules_settings(&b.id).unwrap().config.profile,
        RulesProfile::Revised2024
    );
    assert_eq!(
        serde_json::to_value(reopened.load_characters(&a.id).unwrap()).unwrap(),
        before
    );
    assert_eq!(
        serde_json::to_value(reopened.load_tabletop(&a.id).unwrap().scene).unwrap(),
        serde_json::to_value(scene).unwrap()
    );
    let events = reopened.recent_events(&a.id).unwrap();
    assert_eq!(events[0].event_type, "rules.changed");
    assert_eq!(events[0].payload["config"]["profile"], "5e-2014");
}
#[test]
fn failed_rules_event_rolls_back_profile_change() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Rules atomicity").unwrap().value;
    let conn = connect(&f.0.join(&campaign.slug), false).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_event BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'event failed'); END;").unwrap();
    let mut config = RulesConfig::modern();
    config.profile = RulesProfile::Legacy2014;
    assert!(store.save_rules_settings(&campaign.id, config, 1).is_err());
    let state = store.load_rules_settings(&campaign.id).unwrap();
    assert_eq!(state.revision, 1);
    assert_eq!(state.config.profile, RulesProfile::Revised2024);
}
