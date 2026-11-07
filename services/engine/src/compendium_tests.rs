use super::*;

#[test]
fn nullable_collection_metadata_remains_unassigned_instead_of_blocking_import() {
    let parsed:ContentEntry=serde_json::from_value(serde_json::json!({"name":"Uncatalogued class","description":null,"publisher":null,"book":null,"properties":{}})).unwrap();
    assert_eq!(parsed.description, "");
    assert_eq!(parsed.publisher, "");
    assert_eq!(parsed.book, "");
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Nullable sources").unwrap().value;
    store
        .import_content(
            &campaign.id,
            ContentImport {
                name: "classes.json".into(),
                kind: "classes".into(),
                entries: vec![parsed],
            },
        )
        .unwrap();
    let results = store.search_content(&campaign.id, search()).unwrap();
    assert_eq!(results.entries[0].source_edition, "unassigned");
}
fn entry(name: &str, book: &str) -> ContentEntry {
    ContentEntry {
        name: name.into(),
        description: "<script>reference text</script>".into(),
        properties: serde_json::json!({"Level":1,"Components":"V, S"}),
        publisher: "Example publisher".into(),
        book: book.into(),
    }
}
fn search() -> ContentQuery {
    ContentQuery {
        search: String::new(),
        kind: String::new(),
        edition: String::new(),
        book: String::new(),
        offset: 0,
    }
}
#[test]
fn local_compendium_retains_duplicate_names_and_source_years_after_reopen() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Collection").unwrap().value;
    store
        .import_content(
            &campaign.id,
            ContentImport {
                name: "spells.json".into(),
                kind: "spells".into(),
                entries: vec![
                    entry("Light", "Free Basic Rules (2014)"),
                    entry("Light", "Free Basic Rules (2024)"),
                    entry("Light", "Unmarked book"),
                    entry("100% odd_name", "Other book"),
                ],
            },
        )
        .unwrap();
    drop(store);
    let reopened = f.store();
    let all = reopened.search_content(&campaign.id, search()).unwrap();
    assert_eq!(all.total, 4);
    assert_eq!(all.entries.len(), 4);
    let mut q = search();
    q.search = "Light".into();
    q.edition = "5e-2024".into();
    let selected = reopened.search_content(&campaign.id, q).unwrap();
    assert_eq!(selected.total, 1);
    let details = reopened
        .load_content_entry(&campaign.id, &selected.entries[0].id)
        .unwrap();
    assert_eq!(details.entry.book, "Free Basic Rules (2024)");
    assert_eq!(details.entry.properties["Level"], 1);
    assert_eq!(details.entry.description, "<script>reference text</script>");
    let mut q = search();
    q.search = "%".into();
    assert_eq!(reopened.search_content(&campaign.id, q).unwrap().total, 1);
    let mut q = search();
    q.edition = "unassigned".into();
    assert_eq!(reopened.search_content(&campaign.id, q).unwrap().total, 2);
    assert_eq!(
        reopened.list_content_packs(&campaign.id).unwrap()[0].entry_count,
        4
    );
    let log = fs::read_to_string(f.0.join(&campaign.slug).join("events.log")).unwrap();
    assert!(!log.contains("reference text"));
    assert!(!log.contains("Components"));
}
#[test]
fn compendium_imports_are_atomic_and_campaign_scoped() {
    let f = Fixture::new();
    let store = f.store();
    let a = store.create_campaign("Books A").unwrap().value;
    let b = store.create_campaign("Books B").unwrap().value;
    let saved = store
        .import_content(
            &a.id,
            ContentImport {
                name: "monsters.json".into(),
                kind: "monsters".into(),
                entries: vec![entry("Goblin", "Book")],
            },
        )
        .unwrap();
    let id = store.search_content(&a.id, search()).unwrap().entries[0]
        .id
        .clone();
    assert_eq!(saved.value.entry_count, 1);
    assert!(store.load_content_entry(&b.id, &id).is_err());
    assert_eq!(store.search_content(&b.id, search()).unwrap().total, 0);
    let conn = connect(&f.0.join(&a.slug), false).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_event BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'event failed'); END;").unwrap();
    assert!(store
        .import_content(
            &a.id,
            ContentImport {
                name: "items.json".into(),
                kind: "items".into(),
                entries: vec![entry("Treasure", "Book")]
            }
        )
        .is_err());
    assert_eq!(store.list_content_packs(&a.id).unwrap().len(), 1);
    assert_eq!(store.search_content(&a.id, search()).unwrap().total, 1);
}
#[test]
fn malformed_compendium_files_do_not_create_partial_packs() {
    let f = Fixture::new();
    let store = f.store();
    let campaign = store.create_campaign("Bad import").unwrap().value;
    let mut invalid = entry("Invalid", "Book");
    invalid.properties = serde_json::json!([1, 2, 3]);
    assert!(store
        .import_content(
            &campaign.id,
            ContentImport {
                name: "spells.json".into(),
                kind: "spells".into(),
                entries: vec![entry("Valid", "Book"), invalid]
            }
        )
        .is_err());
    assert!(store
        .import_content(
            &campaign.id,
            ContentImport {
                name: "unknown".into(),
                kind: "executable".into(),
                entries: vec![entry("Text", "Book")]
            }
        )
        .is_err());
    assert!(store.list_content_packs(&campaign.id).unwrap().is_empty());
    assert_eq!(store.recent_events(&campaign.id).unwrap().len(), 1);
}
