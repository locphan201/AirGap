//! Tier 2: Boundary, Resource & Corner Case Integration Tests
//!
//! Verifies boundary conditions: empty files, 1MB notes scalability, deep paths (>260 chars),
//! CRLF vs LF line ending preservation, UTF-8 BOM tolerance, malformed Mermaid graceful fallback,
//! and zip bomb thresholds.

use airgap_core::{
    apply_line_ending, detect_line_ending, import_zip, read_note, sanitize_path, strip_bom,
    write_atomic, ConflictPolicy, ImportError, LineEnding, Vault,
};
use pulldown_cmark::{html, Options, Parser};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[test]
fn test_empty_zero_byte_note_read_write_render() {
    let dir = tempdir().expect("tempdir");
    let vault = Vault::new(dir.path());
    let note_path = dir.path().join("empty.md");

    // Atomic write of 0 bytes
    write_atomic(&note_path, "").expect("Writing 0-byte note should succeed");
    assert!(note_path.exists());
    assert_eq!(fs::metadata(&note_path).unwrap().len(), 0);

    // Read back 0-byte note
    let content = read_note(&note_path).expect("Reading 0-byte note should succeed");
    assert_eq!(content, "");

    // Vault tree scan should represent empty note cleanly
    let tree = vault.scan_tree().expect("scan_tree failed");
    assert_eq!(tree.children.len(), 1);
    assert_eq!(tree.children[0].name, "empty.md");

    // CommonMark parsing of empty content must succeed and yield empty HTML
    let parser = Parser::new("");
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    assert_eq!(html_output, "");
}

#[test]
fn test_large_note_1mb_scalability_and_parsing_latency() {
    let dir = tempdir().expect("tempdir");
    let large_note_path = dir.path().join("large_note.md");

    // Generate ~1.1 MB of structured Markdown text (~22,000 lines)
    let mut large_text = String::with_capacity(1_200_000);
    large_text.push_str("# Performance Benchmark Note\n\n");
    for i in 1..=18_000 {
        large_text.push_str(&format!(
            "Line {}: AirGap local-only Markdown note with **bold text** and `code_snippet_{}`.\n",
            i, i
        ));
    }
    large_text.push_str("\n## Summary Table\n| Item | Value |\n|---|---|\n");
    for i in 1..=2000 {
        large_text.push_str(&format!("| Metric {} | Value {} |\n", i, i * 42));
    }

    assert!(
        large_text.len() > 1_000_000,
        "Generated text must exceed 1 MB, actual size: {} bytes",
        large_text.len()
    );

    // Benchmark atomic write of 1 MB note
    let t_write_start = Instant::now();
    write_atomic(&large_note_path, &large_text).expect("Writing 1 MB note failed");
    let t_write = t_write_start.elapsed();
    println!("1 MB atomic write completed in: {:?}", t_write);

    // Benchmark read of 1 MB note
    let t_read_start = Instant::now();
    let loaded = read_note(&large_note_path).expect("Reading 1 MB note failed");
    let t_read = t_read_start.elapsed();
    println!("1 MB read completed in: {:?}", t_read);
    assert_eq!(loaded.len(), large_text.len());

    // Benchmark CommonMark GFM parsing of 1 MB note (< 500 ms target)
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let t_parse_start = Instant::now();
    let parser = Parser::new_ext(&loaded, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    let t_parse = t_parse_start.elapsed();
    println!("1 MB Markdown GFM parsing completed in: {:?}", t_parse);

    assert!(
        t_parse.as_millis() < 500,
        "1 MB note parsing took {:?}, exceeding 500 ms target latency",
        t_parse
    );
    assert!(html_output.contains("Performance Benchmark Note"));
}

#[test]
fn test_deep_paths_exceeding_260_chars_boundary() {
    let dir = tempdir().expect("tempdir");
    let vault_root = if cfg!(windows) {
        dir.path().canonicalize().unwrap()
    } else {
        dir.path().to_path_buf()
    };
    let vault_root = vault_root.as_path();

    // Build a nested directory path exceeding 260 characters
    // e.g. 15 levels of "subfolder_level_xx" (~300 chars)
    let mut deep_rel = PathBuf::new();
    for i in 1..=15 {
        deep_rel.push(format!("subfolder_level_{:02}", i));
    }
    deep_rel.push("nested_deep_note.md");

    let full_path_str = vault_root.join(&deep_rel).to_string_lossy().into_owned();
    println!("Deep path length: {} chars", full_path_str.len());
    assert!(
        full_path_str.len() > 260,
        "Test requires path length > 260 chars, got {}",
        full_path_str.len()
    );

    // Test sanitization of deep relative path
    let sanitized = sanitize_path(vault_root, &deep_rel);
    assert!(
        sanitized.is_ok(),
        "sanitize_path failed on deep path: {:?}",
        sanitized.err()
    );

    // Write note in deep path
    let deep_abs = sanitized.unwrap();
    let content = "# Deep Note\nSuccessfully persisted in long path.\n";
    let write_res = write_atomic(&deep_abs, content);
    assert!(
        write_res.is_ok(),
        "write_atomic failed on path > 260 chars: {:?}",
        write_res.err()
    );

    // Read back
    let read_back = read_note(&deep_abs).expect("read_note failed on deep path");
    assert_eq!(read_back, content);
}

#[test]
fn test_crlf_vs_lf_line_ending_preservation() {
    // 1. Detection of CRLF vs LF
    let crlf_sample = "Line 1\r\nLine 2\r\nLine 3\r\n";
    let lf_sample = "Line 1\nLine 2\nLine 3\n";

    assert_eq!(detect_line_ending(crlf_sample), LineEnding::Crlf);
    assert_eq!(detect_line_ending(lf_sample), LineEnding::Lf);

    // 2. Conversion and normalization
    let converted_to_crlf = apply_line_ending(lf_sample, LineEnding::Crlf);
    assert_eq!(converted_to_crlf, crlf_sample);

    let converted_to_lf = apply_line_ending(crlf_sample, LineEnding::Lf);
    assert_eq!(converted_to_lf, lf_sample);

    // Mixed line endings normalization
    let mixed_sample = "Header\r\nFirst item\nSecond item\r\n";
    let normalized_lf = apply_line_ending(mixed_sample, LineEnding::Lf);
    assert_eq!(normalized_lf, "Header\nFirst item\nSecond item\n");
}

#[test]
fn test_utf8_bom_tolerance_without_artifact_display() {
    // UTF-8 Byte Order Mark: 0xEF, 0xBB, 0xBF (\u{FEFF})
    let raw_bytes_with_bom = b"\xEF\xBB\xBF# Note with BOM\n\nContent line.";
    let utf8_str = String::from_utf8(raw_bytes_with_bom.to_vec()).unwrap();

    // Verify raw string has BOM
    assert!(utf8_str.starts_with('\u{FEFF}'));

    // Strip BOM
    let clean = strip_bom(&utf8_str);
    assert!(!clean.starts_with('\u{FEFF}'));
    assert_eq!(clean, "# Note with BOM\n\nContent line.");

    // String without BOM should remain unchanged
    let regular_str = "# Normal Note\nContent.";
    assert_eq!(strip_bom(regular_str), regular_str);
}

#[test]
fn test_malformed_mermaid_graceful_fallback_and_zero_panic() {
    let malformed_cases = [
        ("unclosed_bracket", "graph TD\n    A[Unclosed bracket"),
        ("invalid_syntax", "sequenceDiagram\n    participant ->> Bob"),
        ("unknown_diagram", "nonExistentDiagramType\n    foo bar"),
        (
            "pie_without_values",
            "pie title Broken\n    \"No colon value\"",
        ),
        ("empty_graph", "graph TD\n"),
        ("broken_edges", "graph LR\n    A ---> B ---- C"),
    ];

    for (case_name, invalid_code) in malformed_cases {
        // Must never panic; wrap in catch_unwind
        let result = std::panic::catch_unwind(|| mermaid_rs_renderer::render(invalid_code));

        assert!(
            result.is_ok(),
            "Panicked on malformed Mermaid case '{}'!",
            case_name
        );

        let render_res = result.unwrap();
        // It must either return Err or an SVG indicating an error
        if let Err(err) = render_res {
            let err_msg = format!("{:?}", err);
            assert!(
                !err_msg.is_empty(),
                "Error diagnostic message must not be empty for case '{}'",
                case_name
            );
        }
    }
}

#[test]
fn test_zip_bomb_threshold_rejection() {
    let dir = tempdir().expect("tempdir");
    let zip_path = dir.path().join("bomb_test.zip");

    // Create a zip with 5 MB uncompressed data
    {
        let file = File::create(&zip_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("large_payload.bin", options).unwrap();
        // Write 5 MB of zeros
        let chunk = vec![0u8; 1024 * 1024];
        for _ in 0..5 {
            zip.write_all(&chunk).unwrap();
        }
        zip.finish().unwrap();
    }

    // Set max_bytes limit to 2 MB (below the 5 MB payload)
    let max_allowed_bytes = 2 * 1024 * 1024;
    let target_dir = dir.path().join("extracted");
    fs::create_dir_all(&target_dir).unwrap();

    let result = import_zip(
        &zip_path,
        &target_dir,
        ConflictPolicy::Overwrite,
        max_allowed_bytes,
    );

    assert!(
        matches!(result, Err(ImportError::ZipBombExceeded(actual, max)) if actual > max),
        "Expected ZipBombExceeded error, got: {:?}",
        result
    );
}
