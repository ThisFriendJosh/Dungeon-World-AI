//! Opt-in real model check. Uses a fresh, isolated save folder for every run.
use dwa_engine::{CampaignStore, CharacterUpdate, DiceMode, DiceRequest, DmTurnRequest};
use dwa_local_dm::{LocalDm, RuntimeConfig, PROMPT_VERSION};
use std::{path::PathBuf, time::Instant};
use uuid::Uuid;

fn main() -> Result<(), String> {
    let data = std::env::var_os("DWA_DM_SMOKE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("dwa-dm-smoke"));
    let data = data.join(Uuid::new_v4().to_string());
    let store = CampaignStore::new(data.join("campaigns")).map_err(|e| e.to_string())?;
    let campaign = store
        .create_campaign("DM integration test")
        .map_err(|e| e.to_string())?
        .value;
    let character = store
        .create_character(&campaign.id, "Orrek Bearskin", "Barbarian")
        .map_err(|e| e.to_string())?
        .value;
    store.update_character(&campaign.id,&character.id,CharacterUpdate{
        name:character.name,class_name:character.class_name,level:3,
        stats_json:r#"{"str":16,"dex":14,"con":15,"int":8,"wis":12,"cha":10,"hp":19,"maxHp":32,"armorClass":14}"#.into(),
        inventory_json:r#"["Sword (damage not recorded)","Bedroll"]"#.into(),conditions_json:"[]".into(),progression_json:"{}".into(),
    }).map_err(|e|e.to_string())?;
    let dm = LocalDm::new(RuntimeConfig::discover(
        PathBuf::from("."),
        data.join("logs"),
    ));
    let play = |text: &str| -> Result<_, String> {
        let request = DmTurnRequest {
            request_id: Uuid::new_v4().to_string(),
            actor_id: Some(character.id.clone()),
            player_text: text.into(),
            expected_seq: store
                .load_dm_history(&campaign.id)
                .map_err(|e| e.to_string())?
                .latest_event_seq,
        };
        let context = store
            .prepare_dm_context(&campaign.id, &request)
            .map_err(|e| e.to_string())?;
        let start = Instant::now();
        let saved = dm.run_turn_with_tools(
            &context,
            text,
            |tool| {
                store
                    .read_dm_tool(&campaign.id, context.state_seq, tool)
                    .map_err(|e| e.to_string())
            },
            |proposal, model| {
                store
                    .commit_dm_turn(&campaign.id, &request, proposal, model, PROMPT_VERSION)
                    .map_err(|e| e.to_string())
            },
        )?;
        println!(
            "TURN {:.2}s\nPLAYER: {}\nDM: {}\n",
            start.elapsed().as_secs_f64(),
            text,
            saved.value.reply.narration
        );
        Ok(saved.value)
    };
    let first=play("Begin at a ruined gate on a forest road. A wounded gatekeeper has seen the missing caravan. I ask his name and what happened. Do not decide my actions.")?;
    let second=play("I repeat the gatekeeper's name back to him and ask which direction the caravan went. Keep his name consistent with the previous reply.")?;
    let roll = store
        .roll_dice(
            &campaign.id,
            DiceRequest {
                expression: "1d20+3".into(),
                mode: DiceMode::Normal,
                character_id: Some(character.id.clone()),
                label: "Search the road for tracks".into(),
            },
        )
        .map_err(|e| e.to_string())?
        .value;
    println!(
        "ENGINE ROLL: {} = {} (id {})",
        roll.expression, roll.total, roll.id
    );
    play(&format!("I search the road for tracks using recorded roll ID {}. Tell me the recorded total and narrate a cautious, provisional exploration outcome. Do not change my HP or inventory.",roll.id))?;
    let before = store
        .load_characters(&campaign.id)
        .map_err(|e| e.to_string())?;
    drop(store);
    let reopened = CampaignStore::new(data.join("campaigns")).map_err(|e| e.to_string())?;
    let h = reopened
        .load_dm_history(&campaign.id)
        .map_err(|e| e.to_string())?;
    assert_eq!(h.total_turns, 3);
    assert_eq!(h.turns[0].id, first.id);
    assert_eq!(h.turns[1].id, second.id);
    assert_eq!(
        before[0].stats_json,
        reopened
            .load_characters(&campaign.id)
            .map_err(|e| e.to_string())?[0]
            .stats_json
    );
    assert_eq!(
        reopened
            .load_dice_rolls(&campaign.id)
            .map_err(|e| e.to_string())?[0]
            .id,
        roll.id
    );
    dm.shutdown();
    assert!(!dm.status()?.running);
    println!("PASS: 3 real local replies, engine roll, unchanged sheet, save/reopen, clean shutdown.\nTest save: {}",data.display());
    Ok(())
}
