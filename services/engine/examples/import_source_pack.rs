//! Verify a source pack in a fresh isolated campaign, without changing any live save.
use dwa_engine::{CampaignStore, KnowledgeImport};
use serde::Deserialize;
use std::path::PathBuf;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pack {
    schema_version: u32,
    sources: Vec<KnowledgeImport>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(
        args.next()
            .ok_or("Usage: import_source_pack PACK.json TEST_OUTPUT_DIRECTORY")?,
    );
    let output = PathBuf::from(
        args.next()
            .ok_or("Provide an isolated test output directory.")?,
    )
    .join(uuid::Uuid::new_v4().to_string());
    let pack: Pack = serde_json::from_str(&std::fs::read_to_string(input)?)?;
    if pack.schema_version != 1 || pack.sources.is_empty() {
        return Err("Unsupported or empty source pack".into());
    }
    let store = CampaignStore::new(output.join("campaigns"))?;
    let campaign = store.create_campaign("Source pack qualification")?.value;
    let count = pack.sources.len();
    let mut chunks = 0;
    for source in pack.sources {
        let saved = store.import_knowledge(&campaign.id, source)?;
        chunks += saved.value.chunk_count;
        println!(
            "Indexed: {} ({} passages)",
            saved.value.title, saved.value.chunk_count
        );
    }
    drop(store);
    let reopened = CampaignStore::new(output.join("campaigns"))?;
    assert_eq!(reopened.list_knowledge_sources(&campaign.id)?.len(), count);
    for query in ["Mournwell", "Enki", "Ilmater", "Exandria"] {
        let hits = reopened.search_dm_references(&campaign.id, query)?;
        println!("Search {query}: {} matches", hits.len());
    }
    println!(
        "PASS: {count} sources, {chunks} passages, reopen and local search. Test save: {}",
        output.display()
    );
    Ok(())
}
