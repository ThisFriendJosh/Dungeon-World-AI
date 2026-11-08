use super::*;
use crate::art_library::*;
use base64::{engine::general_purpose::STANDARD, Engine};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        Self(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join(format!("art-test-{}", Uuid::new_v4())),
        )
    }
    fn store(&self) -> CampaignStore {
        CampaignStore::new(self.0.clone()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
        if self.0.starts_with(root)
            && self
                .0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("art-test-")
        {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
fn png(w: u32, h: u32) -> String {
    let mut data = vec![];
    {
        let mut encoder = png::Encoder::new(&mut data, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&vec![200; w as usize * h as usize * 4])
            .unwrap();
    }
    format!("data:image/png;base64,{}", STANDARD.encode(data))
}
fn metadata() -> ArtMetadata {
    ArtMetadata {
        title: "Forest bear".into(),
        categories: vec!["Creatures".into(), "Token portraits".into()],
        tags: vec![
            " Forest ".into(),
            "forest".into(),
            "Bear".into(),
            "Aerathis".into(),
        ],
        role: "token".into(),
        note: "A friendly guide".into(),
    }
}
fn request() -> ArtImport {
    ArtImport{metadata:metadata(),data_url:png(64,32),width:64,height:32,thumbnail_data_url:png(32,16),source_svg:Some(r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32"><defs><linearGradient id="gold"><stop offset="0" stop-color="#ffaa00"/></linearGradient></defs><rect width="64" height="32" fill="url(#gold)"/></svg>"##.into())}
}
#[test]
fn artwork_and_multiple_tags_survive_restart_and_remain_campaign_scoped() {
    let f = Fixture::new();
    let store = f.store();
    let a = store.create_campaign("Art A").unwrap().value;
    let b = store.create_campaign("Art B").unwrap().value;
    let expected = request();
    let entry = store.import_art(&a.id, request()).unwrap().value;
    assert_eq!(entry.metadata.tags, vec!["Forest", "Bear", "Aerathis"]);
    assert_eq!(entry.metadata.categories.len(), 2);
    drop(store);
    let store = f.store();
    let entries = store.list_art(&a.id).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].metadata.note, "A friendly guide");
    assert!(entries[0].has_svg);
    assert_eq!(
        store.load_scene_image(&a.id, &entry.id).unwrap(),
        expected.data_url
    );
    assert_eq!(
        store.load_art_preview(&a.id, &entry.id).unwrap(),
        expected.thumbnail_data_url
    );
    assert_eq!(
        store.load_art_svg(&a.id, &entry.id).unwrap(),
        expected.source_svg.unwrap()
    );
    assert!(store.list_art(&b.id).unwrap().is_empty());
    assert!(store.load_art_preview(&b.id, &entry.id).is_err());
    assert!(store.load_art_svg(&b.id, &entry.id).is_err());
    assert!(store
        .update_art(
            &b.id,
            ArtUpdate {
                id: entry.id,
                metadata: metadata()
            }
        )
        .is_err());
}
#[test]
fn rejected_images_and_active_svgs_never_create_partial_assets() {
    let f = Fixture::new();
    let store = f.store();
    let a = store.create_campaign("Art validation").unwrap().value;
    for svg in [
        "<svg><script>bad()</script></svg>",
        "<!DOCTYPE svg [<!ENTITY x SYSTEM 'file:///secret'>]><svg/>",
        "<svg><use href='//example.invalid/image'/></svg>",
        "<svg><rect onload='bad()'/></svg>",
        "<svg><foreignObject/></svg>",
        "<svg><rect style='fill:red'/></svg>",
        "<svg><rect fill='url(#one) url(other)'/></svg>",
        "<svg><g></svg>",
        "<svg/><svg/>",
    ] {
        let mut r = request();
        r.source_svg = Some(svg.into());
        assert!(store.import_art(&a.id, r).is_err(), "{svg}");
    }
    let mut r = request();
    r.data_url = "data:image/png;base64,aW52YWxpZA==".into();
    assert!(store.import_art(&a.id, r).is_err());
    let mut r = request();
    r.width = 4000;
    assert!(store.import_art(&a.id, r).is_err());
    let mut r = request();
    r.thumbnail_data_url = png(512, 512);
    assert!(store.import_art(&a.id, r).is_err());
    assert!(store.list_art(&a.id).unwrap().is_empty());
    assert_eq!(store.recent_events(&a.id).unwrap().len(), 1);
}
#[test]
fn editing_categories_preserves_images_and_rolls_back_with_a_failed_event() {
    let f = Fixture::new();
    let store = f.store();
    let a = store.create_campaign("Tag edits").unwrap().value;
    let entry = store.import_art(&a.id, request()).unwrap().value;
    let original = store.load_scene_image(&a.id, &entry.id).unwrap();
    let conn = connect(&store.locate(&a.id).unwrap(), false).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_art BEFORE INSERT ON events WHEN NEW.event_type='art.tagged' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    let mut changed = metadata();
    changed.title = "Changed".into();
    changed.categories = vec!["GM references".into()];
    assert!(store
        .update_art(
            &a.id,
            ArtUpdate {
                id: entry.id.clone(),
                metadata: changed.clone()
            }
        )
        .is_err());
    assert_eq!(
        store.list_art(&a.id).unwrap()[0].metadata.title,
        "Forest bear"
    );
    conn.execute_batch("DROP TRIGGER fail_art;").unwrap();
    store
        .update_art(
            &a.id,
            ArtUpdate {
                id: entry.id.clone(),
                metadata: changed,
            },
        )
        .unwrap();
    let saved = &store.list_art(&a.id).unwrap()[0];
    assert_eq!(saved.metadata.role, "reference");
    assert_eq!(saved.metadata.title, "Changed");
    assert_eq!(store.load_scene_image(&a.id, &entry.id).unwrap(), original);
    assert!(store.load_art_svg(&a.id, &entry.id).is_ok());
}
#[test]
fn ai_sprite_recipe_png_and_self_contained_svg_are_saved_together() {
    let f = Fixture::new();
    let store = f.store();
    let a = store.create_campaign("AI art").unwrap().value;
    let spec = TokenSpec {
        name: "Forest bear".into(),
        label: "BEAR".into(),
        color: "#345678".into(),
        shape: "hex".into(),
        symbol: "initials".into(),
    };
    let artwork = TokenArtwork {
        png_data_url: png(512, 512),
        recipe: ArtRecipe {
            prompt: "a brown forest bear".into(),
            seed: 42,
            model: "stabilityai/sd-turbo".into(),
            revision: "pinned-test".into(),
            transparent_background: true,
        },
    };
    let svg = token_svg_with_artwork(&spec, Some(&artwork.png_data_url)).unwrap();
    assert!(svg.contains("<image href=\"data:image/png;base64,"));
    assert!(svg.contains("clip-path=\"url(#frame)\""));
    assert!(validate_svg(&svg).is_ok(),"An exported token SVG can be imported again");
    assert!(!svg.contains("<script"));
    let entry = store
        .save_token_asset(
            &a.id,
            TokenAssetRequest {
                spec,
                png_data_url: png(512, 512),
                artwork: Some(artwork),
            },
        )
        .unwrap()
        .value;
    assert_eq!(
        store.list_generated_assets(&a.id).unwrap()[0]
            .recipe
            .as_ref()
            .unwrap()
            .seed,
        42
    );
    let loaded = store.load_generated_asset(&a.id, &entry.id).unwrap();
    assert_eq!(loaded["artwork"]["recipe"]["prompt"], "a brown forest bear");
    assert_eq!(loaded["svg"], svg);
    assert_eq!(
        store.list_art(&a.id).unwrap()[0].metadata.categories,
        vec!["Token portraits"]
    );
    assert!(token_svg_with_artwork(
        &store.list_generated_assets(&a.id).unwrap()[0].spec,
        Some(&png(64, 32))
    )
    .is_err());
}
#[test]
fn invalid_tags_do_not_replace_saved_metadata() {
    let mut m = metadata();
    m.tags = vec!["x".repeat(49)];
    assert!(clean_metadata(m).is_err());
    let mut m = metadata();
    m.categories = vec!["All".into()];
    assert!(clean_metadata(m).is_err());
}
