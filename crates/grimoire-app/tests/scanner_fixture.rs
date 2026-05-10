use grimoire_app::scanner::{scan_library, ScanOptions};
use std::path::PathBuf;

#[tokio::test]
async fn scans_zip_and_rar_files_with_legacy_location() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/library");

    let result = scan_library(ScanOptions {
        source_id: "fixture".to_string(),
        root,
    })
    .await
    .unwrap();

    assert_eq!(result.items.len(), 4);
    assert!(result.items.iter().any(|item| {
        item.file_name == "mixed_sim_strategy.rar"
            && item.legacy_location.as_deref() == Some("SIM+SLG")
            && item.extension.as_deref() == Some("rar")
    }));
}
