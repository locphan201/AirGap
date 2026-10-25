//! Tier 1: Core Feature Integration Tests
//!
//! Verifies atomic writes, path sanitization, Windows reserved names,
//! collision suffix naming, CommonMark/GFM rendering, Mermaid SVG generation,
//! zip-slip rejection, and safe export.

use airgap_core::{
    export_vault_zip, generate_html_document, import_zip, sanitize_path, validate_filename,
    write_atomic, ConflictPolicy, PathSafetyError, Vault,
};
use pulldown_cmark::{html, Options, Parser};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

#[test]
fn test_atomic_write_durability_and_tempfile_cleanup() {
    let dir = tempdir().expect("Failed to create temporary directory");
    let target = dir.path().join("subfolder").join("my_note.md");
    let content = "# Title\n\nThis is a securely written note.\n";

    // Write atomic should create parent directories if needed
    write_atomic(&target, content).expect("write_atomic failed");

    // Verify file exists and content matches exactly
    assert!(target.exists(), "Target file was not created");
    let read_back = fs::read_to_string(&target).expect("Failed to read back note");
    assert_eq!(read_back, content);

    // Verify no temporary files remain in the target directory
    let parent = target.parent().unwrap();
    let entries: Vec<_> = fs::read_dir(parent)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();

    assert_eq!(entries.len(), 1, "Expected only the note file to remain");
    assert_eq!(entries[0], "my_note.md");
}

#[test]
fn test_atomic_write_overwrites_existing_atomically() {
    let dir = tempdir().expect("tempdir");
    let target = dir.path().join("versioned.md");

    write_atomic(&target, "Initial version").expect("write v1");
    assert_eq!(fs::read_to_string(&target).unwrap(), "Initial version");

    write_atomic(&target, "Updated version with more details").expect("write v2");
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "Updated version with more details"
    );
}

#[test]
fn test_path_sanitization_rejects_directory_traversal() {
    let vault = tempdir().expect("tempdir");
    let vault_root = vault.path();

    // Direct parent traversal
    let res1 = sanitize_path(vault_root, Path::new("../outside.md"));
    assert!(
        matches!(res1, Err(PathSafetyError::TraversalAttempted(_))),
        "Expected TraversalAttempted for ../outside.md, got {:?}",
        res1
    );

    // Nested parent traversal
    let res2 = sanitize_path(vault_root, Path::new("nested/../../escaped.md"));
    assert!(
        matches!(res2, Err(PathSafetyError::TraversalAttempted(_))),
        "Expected TraversalAttempted for nested/../../escaped.md, got {:?}",
        res2
    );

    // Deep directory traversal
    let res3 = sanitize_path(vault_root, Path::new("a/b/c/../../../../etc/shadow"));
    assert!(matches!(res3, Err(PathSafetyError::TraversalAttempted(_))));
}

#[test]
fn test_path_sanitization_rejects_absolute_paths_outside_vault() {
    let vault = tempdir().expect("tempdir");
    let vault_root = vault.path();

    let outside_abs = if cfg!(windows) {
        PathBuf::from(r"C:\Windows\System32\notepad.exe")
    } else {
        PathBuf::from("/etc/passwd")
    };

    let res = sanitize_path(vault_root, &outside_abs);
    assert!(
        matches!(res, Err(PathSafetyError::AbsoluteOutsideVault(_))),
        "Expected AbsoluteOutsideVault error, got {:?}",
        res
    );
}

#[test]
fn test_path_sanitization_accepts_valid_relative_paths() {
    let vault = tempdir().expect("tempdir");
    let vault_root = vault.path();

    let valid_rel = Path::new("projects/airgap/architecture.md");
    let sanitized = sanitize_path(vault_root, valid_rel).expect("Valid path should be accepted");

    assert_eq!(sanitized, vault_root.join(valid_rel));
}

#[test]
fn test_windows_reserved_device_names_all_rejected() {
    let reserved_names = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    for name in reserved_names {
        // Uppercase without extension
        assert!(
            matches!(
                validate_filename(name),
                Err(PathSafetyError::ReservedName(_))
            ),
            "Failed to reject uppercase reserved device name: {}",
            name
        );

        // Lowercase without extension
        let lower = name.to_lowercase();
        assert!(
            matches!(
                validate_filename(&lower),
                Err(PathSafetyError::ReservedName(_))
            ),
            "Failed to reject lowercase reserved device name: {}",
            lower
        );

        // With .md extension
        let with_md = format!("{}.md", name);
        assert!(
            matches!(
                validate_filename(&with_md),
                Err(PathSafetyError::ReservedName(_))
            ),
            "Failed to reject reserved name with .md extension: {}",
            with_md
        );

        // Lowercase with extension
        let lower_with_ext = format!("{}.txt", name.to_lowercase());
        assert!(
            matches!(
                validate_filename(&lower_with_ext),
                Err(PathSafetyError::ReservedName(_))
            ),
            "Failed to reject reserved name with .txt extension: {}",
            lower_with_ext
        );
    }

    // Verify valid non-reserved names containing substrings are accepted
    let valid_names = [
        "connie.md",
        "auxiliary.md",
        "null.md",
        "communication.md",
        "prompt.md",
        "printer.md",
    ];

    for valid in valid_names {
        assert!(
            validate_filename(valid).is_ok(),
            "Valid name '{}' was incorrectly rejected",
            valid
        );
    }
}

#[test]
fn test_windows_illegal_characters_and_trailing_rejected() {
    let illegal_names = [
        "note<1>.md",
        "note>2.md",
        "note:3.md",
        "note\"quote\".md",
        "note|pipe.md",
        "note?question.md",
        "note*star.md",
    ];

    for illegal in illegal_names {
        assert!(
            matches!(
                validate_filename(illegal),
                Err(PathSafetyError::IllegalCharacters(_))
            ),
            "Failed to reject illegal filename: {}",
            illegal
        );
    }

    // Trailing dot and trailing space
    assert!(matches!(
        validate_filename("note.md."),
        Err(PathSafetyError::TrailingPeriodOrSpace(_))
    ));
    assert!(matches!(
        validate_filename("note.md "),
        Err(PathSafetyError::TrailingPeriodOrSpace(_))
    ));
    assert!(matches!(
        validate_filename("note."),
        Err(PathSafetyError::TrailingPeriodOrSpace(_))
    ));
    assert!(matches!(
        validate_filename("note "),
        Err(PathSafetyError::TrailingPeriodOrSpace(_))
    ));
}

#[test]
fn test_collision_suffix_naming_untitled() {
    let dir = tempdir().expect("tempdir");
    let vault = Vault::new(dir.path());

    // 1st note: Untitled.md
    let note0 = vault
        .create_note_with_collision_handling(Path::new(""), "Untitled.md")
        .expect("create 1");
    assert_eq!(note0, PathBuf::from("Untitled.md"));
    assert!(dir.path().join("Untitled.md").exists());

    // 2nd note: Untitled 1.md
    let note1 = vault
        .create_note_with_collision_handling(Path::new(""), "Untitled.md")
        .expect("create 2");
    assert_eq!(note1, PathBuf::from("Untitled 1.md"));
    assert!(dir.path().join("Untitled 1.md").exists());

    // 3rd note: Untitled 2.md
    let note2 = vault
        .create_note_with_collision_handling(Path::new(""), "Untitled.md")
        .expect("create 3");
    assert_eq!(note2, PathBuf::from("Untitled 2.md"));
    assert!(dir.path().join("Untitled 2.md").exists());

    // 4th note: Untitled 3.md
    let note3 = vault
        .create_note_with_collision_handling(Path::new(""), "Untitled.md")
        .expect("create 4");
    assert_eq!(note3, PathBuf::from("Untitled 3.md"));
    assert!(dir.path().join("Untitled 3.md").exists());

    // Custom base name collision
    let custom0 = vault
        .create_note_with_collision_handling(Path::new(""), "Meeting.md")
        .expect("create custom");
    assert_eq!(custom0, PathBuf::from("Meeting.md"));

    let custom1 = vault
        .create_note_with_collision_handling(Path::new(""), "Meeting.md")
        .expect("create custom duplicate");
    assert_eq!(custom1, PathBuf::from("Meeting 1.md"));
}

#[test]
fn test_vault_tree_scanning_and_natural_sorting() {
    let dir = tempdir().expect("tempdir");
    let vault = Vault::new(dir.path());

    // Create folders
    vault.create_folder(Path::new(""), "Docs").unwrap();
    vault.create_folder(Path::new(""), "Assets").unwrap();
    vault.create_folder(Path::new(""), "Build").unwrap();

    // Create notes
    vault
        .create_note_with_collision_handling(Path::new(""), "Zeta.md")
        .unwrap();
    vault
        .create_note_with_collision_handling(Path::new(""), "Alpha.md")
        .unwrap();
    vault
        .create_note_with_collision_handling(Path::new(""), "Beta.md")
        .unwrap();

    let tree = vault.scan_tree().expect("scan_tree failed");
    assert_eq!(tree.children.len(), 6);

    // Folders first in alphabetical order: Assets, Build, Docs
    assert_eq!(tree.children[0].name, "Assets");
    assert_eq!(tree.children[1].name, "Build");
    assert_eq!(tree.children[2].name, "Docs");

    // Notes next in alphabetical order: Alpha.md, Beta.md, Zeta.md
    assert_eq!(tree.children[3].name, "Alpha.md");
    assert_eq!(tree.children[4].name, "Beta.md");
    assert_eq!(tree.children[5].name, "Zeta.md");
}

#[test]
fn test_zip_slip_path_traversal_rejection() {
    let temp_vault = tempdir().expect("tempdir");
    let vault_root = temp_vault.path();

    // Construct an in-memory zip archive with path traversal entries
    let zip_path = temp_vault.path().join("malicious.zip");
    {
        let file = File::create(&zip_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // Entry 1: Standard relative escape
        zip.start_file("../../evil.txt", options).unwrap();
        zip.write_all(b"payload 1").unwrap();

        // Entry 2: Deep traversal
        zip.start_file("sub/../../escaped_note.md", options)
            .unwrap();
        zip.write_all(b"payload 2").unwrap();

        zip.finish().unwrap();
    }

    // Extraction must fail due to zip-slip violation or path safety
    let import_target = vault_root.join("imported_vault");
    fs::create_dir_all(&import_target).unwrap();

    let result = import_zip(
        &zip_path,
        &import_target,
        ConflictPolicy::Overwrite,
        10 * 1024 * 1024,
    );

    assert!(
        result.is_err(),
        "Importing malicious zip-slip archive must fail"
    );

    // Verify no files escaped to the parent of import_target
    assert!(
        !vault_root.join("evil.txt").exists(),
        "Zip-slip wrote evil.txt outside extraction root!"
    );
    assert!(
        !vault_root.join("escaped_note.md").exists(),
        "Zip-slip wrote escaped_note.md outside extraction root!"
    );
}

#[test]
fn test_vault_zip_export_and_content_preservation() {
    let dir = tempdir().expect("tempdir");
    let vault_root = dir.path().join("source_vault");
    fs::create_dir_all(&vault_root).unwrap();

    // Populate notes and subfolders
    let note1 = vault_root.join("Overview.md");
    let subfolder = vault_root.join("Guides");
    fs::create_dir_all(&subfolder).unwrap();
    let note2 = subfolder.join("Getting_Started.md");

    fs::write(&note1, "# Overview Content").unwrap();
    fs::write(&note2, "## Step 1: Install Rust").unwrap();

    // Export to zip
    let zip_dest = dir.path().join("vault_backup.zip");
    export_vault_zip(&vault_root, &zip_dest).expect("export_vault_zip failed");

    assert!(zip_dest.exists(), "Backup zip file was not created");

    // Inspect zip contents
    let zip_file = File::open(&zip_dest).unwrap();
    let mut archive = ZipArchive::new(zip_file).unwrap();

    let mut names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect();
    names.sort();

    assert!(
        names.iter().any(|n| n == "Overview.md"),
        "Overview.md missing in zip: {:?}",
        names
    );
    assert!(
        names.iter().any(|n| n == "Guides/Getting_Started.md"),
        "Guides/Getting_Started.md missing in zip: {:?}",
        names
    );
}

#[test]
fn test_standalone_html_export_generation() {
    let title = "Architecture Specification";
    let body_html = "<h2>Section 1</h2><p>AirGap is offline and native.</p>";

    let html_doc = generate_html_document(title, body_html);

    assert!(html_doc.contains("<!DOCTYPE html>"));
    assert!(html_doc.contains("<meta charset=\"UTF-8\">"));
    assert!(html_doc.contains(&format!("<title>{}</title>", title)));
    assert!(html_doc.contains(&format!("<h1>{}</h1>", title)));
    assert!(html_doc.contains(body_html));
    assert!(html_doc.contains("<style>"));
}

#[test]
fn test_commonmark_gfm_rendering() {
    let markdown_input = r#"# Main Heading

This is a **bold** and *italic* and ~~strikethrough~~ note with `inline_code`.

## Lists & Tasks
- [x] Completed task item
- [ ] Incomplete task item
- Regular list item

## Table
| Header 1 | Header 2 |
|:---------|:--------:|
| Cell A   | Cell B   |

```rust
fn main() {
    println!("Hello, AirGap!");
}
```

> Blockquote quotation text.

---
"#;

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(markdown_input, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // Verify key rendered elements
    assert!(html_output.contains("<h1>Main Heading</h1>"));
    assert!(html_output.contains("<strong>bold</strong>"));
    assert!(html_output.contains("<em>italic</em>"));
    assert!(html_output.contains("<del>strikethrough</del>"));
    assert!(html_output.contains("<code>inline_code</code>"));
    assert!(html_output.contains("<input disabled=\"\" type=\"checkbox\" checked=\"\"/>"));
    assert!(html_output.contains("<input disabled=\"\" type=\"checkbox\"/>"));
    assert!(html_output.contains("<table>"));
    assert!(html_output.contains("Header 1</th>"));
    assert!(html_output.contains("Cell A</td>"));
    assert!(html_output.contains("<code class=\"language-rust\">"));
    assert!(html_output.contains("<blockquote>"));
    assert!(html_output.contains("<hr />"));
}

#[test]
fn test_mermaid_diagram_svg_generation_all_7_types() {
    // 7 diagram types verified by Spike M0
    let diagrams = [
        ("flowchart", "graph TD\n    A[Start] --> B{Decide}\n    B -- Yes --> C[Proceed]\n    B -- No --> D[Stop]"),
        ("sequence", "sequenceDiagram\n    participant Alice\n    participant Bob\n    Alice->>Bob: Hello Bob\n    Bob-->>Alice: Hi Alice"),
        ("class", "classDiagram\n    class Account {\n        +String id\n        +deposit(amount)\n    }"),
        ("state", "stateDiagram-v2\n    [*] --> Idle\n    Idle --> Running : start\n    Running --> [*] : finish"),
        ("er", "erDiagram\n    CUSTOMER ||--o{ ORDER : places\n    ORDER ||--|{ LINE_ITEM : contains"),
        ("pie", "pie title Distribution\n    \"A\" : 60\n    \"B\" : 30\n    \"C\" : 10"),
        ("gantt", "gantt\n    title Project\n    dateFormat YYYY-MM-DD\n    section Core\n    Task : 2026-01-01, 10d"),
    ];

    for (name, source) in diagrams {
        let render_result = mermaid_rs_renderer::render(source);
        assert!(
            render_result.is_ok(),
            "Native Mermaid rendering failed for type '{}': {:?}",
            name,
            render_result.err()
        );

        let svg = render_result.unwrap();
        assert!(
            svg.contains("<svg"),
            "Output for '{}' does not contain opening <svg tag",
            name
        );
        assert!(
            svg.contains("</svg>"),
            "Output for '{}' does not contain closing </svg> tag",
            name
        );
    }
}
