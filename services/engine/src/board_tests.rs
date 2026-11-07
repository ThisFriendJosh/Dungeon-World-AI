use super::*;

#[test]
fn bundled_art_ids_and_background_alignment_survive_reopening() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Bundled artwork").unwrap().value;
    let catalog: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packages/shared/assets/catalog.json"
    )))
    .unwrap();
    let assets = catalog["assets"].as_array().unwrap();
    let background = assets
        .iter()
        .find(|a| a["title"] == "Forest trail junction")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let sprite = assets
        .iter()
        .find(|a| a["sourceIndex"] == 49 && a["role"] == "portrait")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let mut scene = board_scene();
    let board = scene.board.as_mut().unwrap();
    board.background = Some(background.into());
    board.tokens[0].sprite = Some(sprite.into());
    board.background_fit = "stretch".into();
    board.background_scale = 1.2;
    board.background_x = 0.3;
    board.background_y = -0.1;
    store.save_tabletop(&campaign.id, scene, 0).unwrap();
    drop(store);
    let board = f
        .store()
        .load_tabletop(&campaign.id)
        .unwrap()
        .scene
        .unwrap()
        .board
        .unwrap();
    assert_eq!(board.background.as_deref(), Some(background));
    assert_eq!(board.tokens[0].sprite.as_deref(), Some(sprite));
    assert_eq!(board.background_fit, "stretch");
    assert_eq!(board.background_scale, 1.2);
    assert_eq!(board.background_x, 0.3);
}

fn board_scene() -> FariScene {
    let mut scene = test_scene();
    scene.board=Some(serde_json::from_value(serde_json::json!({
   "columns":30,"rows":20,"feetPerCell":5,"grid":true,"snap":true,"gridOpacity":0.22,
   "background":"builtin:pond","tokens":[{
      "id":Uuid::new_v4().to_string(),"name":"Bearskin","kind":"player","x":4.0,"y":6.0,"size":1,"color":"#d6b973",
      "sprite":null,"characterId":null,"notes":"Guard the bridge.","states":["Prone"],"hp":8,"maxHp":15,
      "hidden":false,"locked":false,"rotation":15.0,"brightLight":20.0,"dimLight":40.0
   }],"drawings":[{"id":Uuid::new_v4().to_string(),"kind":"pen","points":[{"x":1.0,"y":1.0},{"x":3.0,"y":2.0}],"color":"#abcdef","width":0.1}],
   "fogEnabled":true,"fog":[{"id":Uuid::new_v4().to_string(),"x":4.5,"y":6.5,"radius":2.0,"reveal":true}],"ambient":0.0
 })).unwrap());
    scene
}
fn tiny_image() -> ImageImport {
    ImageImport { name:"One pixel map".into(), width:1,height:1,
 data_url:"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Y9ZlXkAAAAASUVORK5CYII=".into() }
}
#[test]
fn map_tokens_drawings_fog_and_light_survive_scene_switches_and_reopen() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Map collection").unwrap().value;
    let first = board_scene();
    let first_id = first.id.clone();
    let expected = serde_json::to_value(&first).unwrap();
    store.save_tabletop(&campaign.id, first, 0).unwrap();
    let mut second = board_scene();
    second.name = "The sewer".into();
    second.board.as_mut().unwrap().background = Some("builtin:sewer".into());
    store.save_tabletop(&campaign.id, second, 1).unwrap();
    assert_eq!(store.list_tabletop_scenes(&campaign.id).unwrap().len(), 2);
    let selected = store
        .select_tabletop_scene(&campaign.id, &first_id, 2)
        .unwrap()
        .value;
    assert_eq!(selected.revision, 3);
    assert_eq!(serde_json::to_value(&selected.scene).unwrap(), expected);
    assert!(store
        .select_tabletop_scene(&campaign.id, &first_id, 2)
        .is_err());
    drop(store);
    let reopened = f.store();
    assert_eq!(
        serde_json::to_value(&reopened.load_tabletop(&campaign.id).unwrap().scene).unwrap(),
        expected
    );
    assert_eq!(
        reopened.list_tabletop_scenes(&campaign.id).unwrap().len(),
        2
    );
}
#[test]
fn legacy_single_scene_is_added_to_library_without_changing_cards() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Legacy scene").unwrap().value;
    let scene = test_scene();
    let expected = serde_json::to_value(&scene).unwrap();
    store.save_tabletop(&campaign.id, scene, 0).unwrap();
    let conn = connect(&f.0.join(&campaign.slug), false).unwrap();
    conn.execute_batch("DROP TABLE tabletop_scenes; DROP TABLE scene_images;")
        .unwrap();
    drop(conn);
    drop(store);
    let reopened = f.store();
    assert_eq!(
        reopened.list_tabletop_scenes(&campaign.id).unwrap().len(),
        1
    );
    assert_eq!(
        serde_json::to_value(&reopened.load_tabletop(&campaign.id).unwrap().scene).unwrap(),
        expected
    );
}
#[test]
fn invalid_board_coordinates_identifiers_images_and_lighting_do_not_save() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Invalid map").unwrap().value;
    for change in 0..8 {
        let mut scene = board_scene();
        let card_id = scene.index_cards.public[0].id.clone();
        let b = scene.board.as_mut().unwrap();
        match change {
            0 => b.feet_per_cell = 10,
            1 => b.tokens[0].x = 30.0,
            2 => b.tokens[0].x = f64::NAN,
            3 => b.tokens[0].id = card_id,
            4 => b.tokens[0].dim_light = 10.0,
            5 => b.background = Some("remote:image".into()),
            6 => b.background = Some(Uuid::new_v4().to_string()),
            _ => b.fog[0].radius = 99.0,
        }
        assert!(store.save_tabletop(&campaign.id, scene, 0).is_err());
    }
    assert!(store.list_tabletop_scenes(&campaign.id).unwrap().is_empty());
    assert_eq!(store.recent_events(&campaign.id).unwrap().len(), 1);
}
#[test]
fn imported_images_and_scenes_are_isolated_between_campaigns() {
    let f = Fixture::new();
    let store = f.store();
    let a = store.create_campaign("Image owner").unwrap().value;
    let b = store.create_campaign("Other map").unwrap().value;
    let request = tiny_image();
    let expected = request.data_url.clone();
    let imported = store.import_scene_image(&a.id, request).unwrap().value;
    assert_eq!(
        store.load_scene_image(&a.id, &imported.id).unwrap(),
        expected
    );
    assert!(store.load_scene_image(&b.id, &imported.id).is_err());
    assert!(store.list_scene_images(&b.id).unwrap().is_empty());
    let mut scene = board_scene();
    scene.board.as_mut().unwrap().background = Some(imported.id.clone());
    assert!(store.save_tabletop(&b.id, scene.clone(), 0).is_err());
    scene.board.as_mut().unwrap().tokens[0].sprite = Some(imported.id.clone());
    let id = scene.id.clone();
    store.save_tabletop(&a.id, scene, 0).unwrap();
    assert!(store.select_tabletop_scene(&b.id, &id, 0).is_err());
    drop(store);
    let reopened = f.store();
    assert_eq!(
        reopened.load_scene_image(&a.id, &imported.id).unwrap(),
        expected
    );
    assert_eq!(reopened.list_scene_images(&a.id).unwrap().len(), 1);
    let log = fs::read_to_string(f.0.join(&a.slug).join("events.log")).unwrap();
    assert!(!log.contains("iVBORw0"));
}
#[test]
fn image_import_rejects_invalid_data_and_dimensions_without_events() {
    let f = Fixture::new();
    let store = f.store();
    let c = store.create_campaign("Invalid images").unwrap().value;
    let mut request = tiny_image();
    request.width = 2;
    assert!(store.import_scene_image(&c.id, request).is_err());
    let mut request = tiny_image();
    request.data_url = "data:image/svg+xml;base64,AA==".into();
    assert!(store.import_scene_image(&c.id, request).is_err());
    let mut request = tiny_image();
    request.data_url = "data:image/png;base64,bad".into();
    assert!(store.import_scene_image(&c.id, request).is_err());
    assert!(store.list_scene_images(&c.id).unwrap().is_empty());
    assert_eq!(store.recent_events(&c.id).unwrap().len(), 1);
}
#[test]
fn scene_library_and_selection_roll_back_when_event_insert_fails() {
    let f = Fixture::new();
    let store = f.store();
    let c = store.create_campaign("Map atomicity").unwrap().value;
    let first = board_scene();
    let first_id = first.id.clone();
    store.save_tabletop(&c.id, first, 0).unwrap();
    let second = board_scene();
    let second_id = second.id.clone();
    store.save_tabletop(&c.id, second, 1).unwrap();
    let conn = connect(&f.0.join(&c.slug), false).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_map_event BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'no event'); END;").unwrap();
    assert!(store.select_tabletop_scene(&c.id, &first_id, 2).is_err());
    assert!(store.save_tabletop(&c.id, board_scene(), 2).is_err());
    assert_eq!(store.list_tabletop_scenes(&c.id).unwrap().len(), 2);
    let active = store.load_tabletop(&c.id).unwrap();
    assert_eq!(active.revision, 2);
    assert_eq!(active.scene.unwrap().id, second_id);
}
