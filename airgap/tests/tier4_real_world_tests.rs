//! Tier 4: Real-World End-to-End User Scenario Tests
//!
//! Simulates realistic complete user workflows:
//! 1. Initializing a new vault with nested directory hierarchy.
//! 2. Writing rich Markdown with GFM tables, checklists, code fences, and embedded Mermaid diagrams.
//! 3. Exporting the document to standalone single-file HTML with embedded CSS and inline SVG graphics.
//! 4. Creating a full vault backup to `.zip`, destroying the source directory, and performing 100% loss-free round-trip disaster recovery.

use airgap_core::{
    export_vault_zip, generate_html_document, import_zip, read_note, write_atomic, ConflictPolicy,
    Vault,
};
use pulldown_cmark::{html, Options, Parser};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn test_real_world_vault_initialization_and_hierarchy() {
    let dir = tempdir().expect("tempdir");
    let vault = Vault::new(dir.path());

    // User creates project folders
    let arch_dir = vault
        .create_folder(Path::new(""), "Architecture")
        .expect("create Architecture");
    let rfc_dir = vault
        .create_folder(Path::new(""), "RFCs")
        .expect("create RFCs");
    let docs_dir = vault
        .create_folder(&arch_dir, "Diagrams")
        .expect("create Architecture/Diagrams");

    assert_eq!(arch_dir, PathBuf::from("Architecture"));
    assert_eq!(rfc_dir, PathBuf::from("RFCs"));
    assert_eq!(docs_dir, PathBuf::from("Architecture").join("Diagrams"));

    let note1 = vault
        .create_note_with_collision_handling(&arch_dir, "System_Overview.md")
        .unwrap();
    let note2 = vault
        .create_note_with_collision_handling(&docs_dir, "Component_Flow.md")
        .unwrap();
    let note3 = vault
        .create_note_with_collision_handling(&rfc_dir, "RFC_001_Zero_Network.md")
        .unwrap();

    assert_eq!(
        note1,
        PathBuf::from("Architecture").join("System_Overview.md")
    );
    assert_eq!(
        note2,
        PathBuf::from("Architecture")
            .join("Diagrams")
            .join("Component_Flow.md")
    );
    assert_eq!(note3, PathBuf::from("RFCs").join("RFC_001_Zero_Network.md"));

    // Verify tree representation
    let tree = vault.scan_tree().expect("scan_tree failed");
    assert_eq!(tree.children.len(), 2); // Architecture, RFCs
    assert_eq!(tree.children[0].name, "Architecture");
    assert_eq!(tree.children[1].name, "RFCs");

    // Check Architecture folder children
    let arch_children = &tree.children[0].children;
    assert_eq!(arch_children.len(), 2); // Diagrams (folder) + System_Overview.md (note)
    assert_eq!(arch_children[0].name, "Diagrams");
    assert_eq!(arch_children[1].name, "System_Overview.md");
}

#[test]
fn test_real_world_authoring_html_export_with_inline_svg() {
    let dir = tempdir().expect("tempdir");
    let vault = Vault::new(dir.path());

    // 1. User authors a rich technical document with an embedded Mermaid diagram
    let markdown_body = r#"# Architecture Specification: AirGap Engine

AirGap is an **offline-first**, native Markdown environment designed for *security* and *speed*.

## System Status Checklist
- [x] Zero network dependency audit passed
- [x] Atomic write protocol verified
- [ ] Multi-platform packaging complete

## Performance Benchmarks
| Component | Metric | Status |
|:---|:---:|---:|
| Storage | < 5 ms atomic write | PASS |
| Parser | < 500 ms for 1 MB | PASS |
| Mermaid | < 10 ms native SVG | PASS |

## Core Pipeline Architecture
```mermaid
graph LR
    Editor[TextEdit Multiline] --> AST[pulldown-cmark Parser]
    AST --> DocModel[Intermediate Representation]
    DocModel --> Resvg[tiny-skia / resvg Rasterizer]
    Resvg --> Texture[egui GPU Texture]
```

## Source Code Reference
```rust
fn execute_airgap() {
    println!("Zero network, true local storage.");
}
```
"#;

    let note_rel = vault
        .create_note_with_collision_handling(Path::new(""), "TechSpec.md")
        .unwrap();
    let note_abs = dir.path().join(&note_rel);
    write_atomic(&note_abs, markdown_body).unwrap();

    // 2. Render Markdown content
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown_body, options);
    let mut rendered_html = String::new();
    html::push_html(&mut rendered_html, parser);

    // 3. Render embedded Mermaid diagram to native SVG
    let mermaid_code = r#"graph LR
    Editor[TextEdit Multiline] --> AST[pulldown-cmark Parser]
    AST --> DocModel[Intermediate Representation]
    DocModel --> Resvg[tiny-skia / resvg Rasterizer]
    Resvg --> Texture[egui GPU Texture]"#;

    let svg_result = mermaid_rs_renderer::render(mermaid_code);
    assert!(svg_result.is_ok(), "Native Mermaid SVG generation failed");
    let svg_markup = svg_result.unwrap();

    // 4. Inject SVG into exported HTML document (replacing ```mermaid code block)
    let processed_body = rendered_html.replace(
        "<code class=\"language-mermaid\">",
        "<div class=\"mermaid-diagram\">",
    );
    let final_body = format!(
        "{}\n<div class=\"svg-wrapper\">{}</div>",
        processed_body, svg_markup
    );

    // 5. Generate standalone HTML
    let standalone_html = generate_html_document("TechSpec — AirGap", &final_body);

    let export_path = dir.path().join("TechSpec_Export.html");
    write_atomic(&export_path, &standalone_html).unwrap();

    // 6. Inspect exported HTML document
    assert!(export_path.exists());
    let exported_text = fs::read_to_string(&export_path).unwrap();

    assert!(exported_text.contains("<!DOCTYPE html>"));
    assert!(exported_text.contains("<title>TechSpec — AirGap</title>"));
    assert!(exported_text.contains("Architecture Specification: AirGap Engine"));
    assert!(exported_text.contains("<table>"));
    assert!(exported_text.contains("<svg"));
    assert!(exported_text.contains("</svg>"));
    assert!(
        !exported_text.contains("<script src="),
        "Exported standalone HTML must not reference external scripts"
    );
    assert!(
        !exported_text.contains("<link rel=\"stylesheet\""),
        "Exported standalone HTML must not reference external stylesheets"
    );
}

#[test]
fn test_real_world_backup_disaster_recovery_round_trip() {
    let dir = tempdir().expect("tempdir");
    let original_vault = dir.path().join("my_active_vault");
    fs::create_dir_all(&original_vault).unwrap();

    // Populate active vault with multi-level directories and varied notes
    let notes = [
        ("Readme.md", "# Active Vault\nWelcome to my notes."),
        (
            "Projects/AirGap/Roadmap.md",
            "## 2026 Roadmap\n- Feature complete.",
        ),
        (
            "Projects/AirGap/Spec.md",
            "### Specifications\nOffline storage.",
        ),
        (
            "Journal/2026-10-24.md",
            "Day 1 of AirGap development.\nWent great.",
        ),
        ("Assets/Notes.txt", "Plain text notes inside assets."),
    ];

    for (rel_path, content) in &notes {
        let abs_path = original_vault.join(rel_path);
        fs::create_dir_all(abs_path.parent().unwrap()).unwrap();
        write_atomic(&abs_path, content).unwrap();
    }

    // 1. User performs a full vault backup export to .zip
    let backup_zip = dir.path().join("airgap_backup_20261024.zip");
    export_vault_zip(&original_vault, &backup_zip).expect("export_vault_zip failed");
    assert!(backup_zip.exists(), "Backup archive was not created");

    // 2. DISASTER SIMULATION: Wipe the original vault completely from disk!
    fs::remove_dir_all(&original_vault).expect("Failed to delete original vault");
    assert!(!original_vault.exists(), "Original vault was not deleted");

    // 3. RECOVERY: Initialize a fresh vault and restore from the backup archive
    let restored_vault = dir.path().join("restored_vault");
    fs::create_dir_all(&restored_vault).unwrap();

    let summary = import_zip(
        &backup_zip,
        &restored_vault,
        ConflictPolicy::Overwrite,
        50 * 1024 * 1024,
    )
    .expect("import_zip disaster recovery failed");

    assert_eq!(
        summary.files_imported,
        notes.len(),
        "Recovered file count mismatch"
    );

    // 4. VERIFICATION: Assert 100% bit-for-bit fidelity for all restored files
    for (rel_path, expected_content) in &notes {
        let restored_file = restored_vault.join(rel_path);
        assert!(
            restored_file.exists(),
            "Restored file missing: {:?}",
            restored_file
        );
        let restored_content = read_note(&restored_file).expect("Failed to read restored file");
        assert_eq!(
            restored_content, *expected_content,
            "Restored content mismatch in {:?}",
            rel_path
        );
    }
}
