use dwa_engine::{CampaignStore, ContentEntry, ContentImport, ContentQuery};
use std::{env, fs, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("Usage: import_compendium <campaigns-root> <existing-campaign-slug | --new-audit> <extracted-data-folder>".into());
    }
    let store = CampaignStore::new(PathBuf::from(&args[0]))?;
    let campaign = if args[1] == "--new-audit" {
        store.create_campaign("Compendium import audit")?.value
    } else {
        store.load_campaign(&args[1])?.value
    };
    for kind in [
        "backgrounds",
        "classes",
        "items",
        "monsters",
        "species",
        "spells",
    ] {
        let path = PathBuf::from(&args[2]).join(format!("{kind}.json"));
        if !path.is_file() {
            continue;
        }
        let entries: Vec<ContentEntry> = serde_json::from_slice(&fs::read(path)?)?;
        let result = store.import_content(
            &campaign.id,
            ContentImport {
                name: format!("{kind}.json"),
                kind: kind.into(),
                entries,
            },
        )?;
        println!(
            "{}: {} imported{}",
            result.value.kind,
            result.value.entry_count,
            result.warning.map(|v| format!("; {v}")).unwrap_or_default()
        );
    }
    let all = store.search_content(
        &campaign.id,
        ContentQuery {
            search: String::new(),
            kind: String::new(),
            edition: String::new(),
            book: String::new(),
            offset: 0,
        },
    )?;
    println!(
        "{} total entries in {}. Source books: {}",
        all.total,
        campaign.slug,
        store.list_content_books(&campaign.id)?.len()
    );
    Ok(())
}
