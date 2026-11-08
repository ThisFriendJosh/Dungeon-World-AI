//! Create a fresh local UI fixture using the public bundled SRD; never touches a live campaign.
use base64::{engine::general_purpose::STANDARD, Engine};
use dwa_engine::{
    CalculationOperation, CalculationRequest, CampaignStore, KnowledgeImport,
    OriginalDocumentImport,
};
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Provide an isolated QA output directory.")?,
    )
    .join(uuid::Uuid::new_v4().to_string());
    let store = CampaignStore::new(output.join("campaigns"))?;
    let campaign = store.create_campaign("Document reader verification")?.value;
    let rules = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/vtt-ui/public/rules");
    let pages: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(rules.join("srd-5.2.1.json"))?)?;
    let text = pages["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| format!("[Page {}]\n{}\n", p["number"], p["text"].as_str().unwrap()))
        .collect::<String>();
    store.import_knowledge_with_original(
        &campaign.id,
        KnowledgeImport {
            title: "Original PDF reader check".into(),
            text,
            source_ref: "Bundled public SRD 5.2.1".into(),
            edition: "5e-2024".into(),
            gm_only: false,
        },
        Some(OriginalDocumentImport {
            name: "srd-5.2.1.pdf".into(),
            data_base64: STANDARD.encode(std::fs::read(rules.join("srd-5.2.1.pdf"))?),
        }),
    )?;
    store.import_knowledge(
        &campaign.id,
        KnowledgeImport {
            title: "Older text-only notes".into(),
            text: format!(
                "{}\nPassage tail verification.",
                "The wounded gatekeeper waits near the eastern gate.\n".repeat(400)
            ),
            source_ref: "Synthetic QA notes".into(),
            edition: "homebrew".into(),
            gm_only: true,
        },
    )?;
    let actor = store
        .create_character(&campaign.id, "Reader tester", "Fighter")?
        .value;
    store.calculate_campaign(
        &campaign.id,
        CalculationRequest {
            character_id: Some(actor.id),
            operation: CalculationOperation::Arithmetic {
                expression: "(12+8)/2".into(),
            },
        },
    )?;
    println!("QA_DATA_DIR={}", output.display());
    Ok(())
}
