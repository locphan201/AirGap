//! Tier 5 Adversarial Hardening Test Suite (Milestone M0 Verification)
//!
//! Comprehensive empirical stress testing for:
//! 1. Native Mermaid Rendering across all 7 diagram types (Flowchart, Sequence, Class, State, ER, Pie, Gantt)
//!    under both Light and Dark themes, rasterized at 1.0x, 1.5x, and 2.0x HiDPI scale.
//! 2. Heavily malformed Mermaid syntax (empty strings, random garbage, deeply nested loops, cyclic transitions,
//!    unclosed quotes/brackets, script injection attempts, unsupported keywords, and massive graph stress)
//!    verifying graceful error fallback with zero panics and zero deadlocks.
//! 3. Path safety in crates/core verifying Windows reserved names (CON, AUX, NUL, COM1-9, LPT1-9), multi-extension
//!    variations, directory traversal (Unix and Windows styles), illegal characters, control characters, and trailing
//!    dots/spaces.

use airgap_core::path_safety::{sanitize_path, validate_filename, PathSafetyError};
use airgap_render::fallback::DiagramErrorFallback;
use airgap_render::mermaid::{
    detect_diagram_type, render_mermaid_to_svg, DiagramType, MermaidError,
};
use airgap_render::raster::rasterize_svg_to_pixmap;
use std::path::Path;
use std::time::Instant;
use tempfile::tempdir;

// ============================================================================
// 1. NATIVE MERMAID RENDERING: ALL 7 DIAGRAM TYPES (DEEP SPECIFICATION)
// ============================================================================

#[test]
fn test_adversarial_mermaid_all_7_types_dual_themes_and_hidpi() {
    let test_cases = [
        (
            "flowchart_advanced",
            DiagramType::Flowchart,
            r#"flowchart TB
                subgraph Core ["Core Engine"]
                    direction LR
                    A["Start Node [Input]"] -->|Process| B("Round Decision")
                    B --> C{"Check Condition"}
                    C -->|Yes| D[("Database Cylinder")]
                    C -->|No| E>(Asymmetric Flag)
                end
                subgraph UI ["User Interface"]
                    F((Circle Entry)) --> G{{Hexagon Display}}
                end
                Core --> UI
                D -.->|Async Sync| F
            "#,
        ),
        (
            "sequence_advanced",
            DiagramType::Sequence,
            r#"sequenceDiagram
                autonumber
                actor User as Alice
                participant Client as AirGap Client
                participant Core as Core Engine
                participant Storage as Disk Storage

                User->>Client: Open Vault
                activate Client
                Client->>Core: Scan Tree
                activate Core
                Core->>Storage: Read Inode Metadata
                Storage-->>Core: Directory Entries
                deactivate Core
                opt Unsaved Changes Exist
                    Client-->>User: Show 3-Way Conflict Prompt
                end
                loop Every 1s Autosave
                    Core->>Storage: Atomic Flush (.tmp -> fsync -> rename)
                end
                deactivate Client
            "#,
        ),
        (
            "class_advanced",
            DiagramType::Class,
            r#"classDiagram
                class Vault {
                    +PathBuf root
                    +scan_tree() TreeNode
                    +write_atomic(path, content)
                    -validate_safety(path) bool
                }
                class TreeNode {
                    +String name
                    +PathBuf relative_path
                    +bool is_dir
                    +Vec children
                }
                class FileWatcher {
                    +watch(path)
                    +unwatch(path)
                }
                Vault *-- TreeNode : aggregates
                Vault --|> FileWatcher : inherits
            "#,
        ),
        (
            "state_advanced",
            DiagramType::State,
            r#"stateDiagram-v2
                [*] --> Idle
                state Editing {
                    [*] --> Clean
                    Clean --> Dirty : Keystroke
                    Dirty --> Saving : Autosave Trigger (~1s)
                    Saving --> Clean : Atomic Write Success
                    Saving --> Conflict : External Modification Detected
                }
                Idle --> Editing : Select Note
                Editing --> Idle : Close Note
                Conflict --> Clean : Resolve (Keep Mine / Theirs)
            "#,
        ),
        (
            "er_advanced",
            DiagramType::Er,
            r#"erDiagram
                VAULT ||--o{ NOTE : contains
                NOTE ||--|{ REVISION : tracks
                NOTE }o--o{ TAG : categorized_by
                VAULT {
                    string id PK
                    string root_path
                    datetime created_at
                }
                NOTE {
                    string path PK
                    string title
                    int byte_size
                    boolean is_pinned
                }
                TAG {
                    string name PK
                    string color_hex
                }
            "#,
        ),
        (
            "pie_advanced",
            DiagramType::Pie,
            r#"pie title Vault Content Distribution
                "Markdown Notes (.md)" : 65.5
                "Embedded Images (PNG/JPG)" : 20.0
                "Mermaid Diagrams" : 10.0
                "Attachments & Others" : 4.5
            "#,
        ),
        (
            "gantt_advanced",
            DiagramType::Gantt,
            r#"gantt
                title AirGap Development Roadmap
                dateFormat YYYY-MM-DD
                section Architecture & Feasibility
                    Mermaid & Editor Spike (M0)    :done, m0, 2026-10-01, 2026-10-10
                    Adversarial Hardening Verification:active, m0_verif, 2026-10-11, 2026-10-15
                section Core Engineering
                    Workspace & Atomic I/O (M1)    :crit, m1, 2026-10-16, 2026-10-25
                    Markdown & Live Preview (M2)   :m2, after m1, 10d
                section Native Integration
                    Mermaid Render Cache Pipeline (M3):m3, after m2, 12d
                    Search, Trash & Watcher (M4)   :m4, after m3, 14d
            "#,
        ),
    ];

    for (name, expected_type, source) in test_cases {
        let detected = detect_diagram_type(source);
        assert_eq!(
            detected, expected_type,
            "Diagram type detection mismatch for '{}': expected {:?}, got {:?}",
            name, expected_type, detected
        );

        // Test dual themes: Light & Dark
        for is_dark in [false, true] {
            let t_start = Instant::now();
            let svg_result = render_mermaid_to_svg(source, is_dark);
            let duration = t_start.elapsed();

            assert!(
                svg_result.is_ok(),
                "Failed to render SVG for '{}' (is_dark: {}): {:?}",
                name,
                is_dark,
                svg_result.err()
            );

            let svg = svg_result.unwrap();
            assert!(
                svg.contains("<svg"),
                "Output for '{}' missing <svg tag: {}",
                name,
                &svg[..100.min(svg.len())]
            );
            assert!(
                svg.contains("</svg>"),
                "Output for '{}' missing closing </svg>",
                name
            );

            println!(
                "Rendered '{}' (is_dark: {}, size: {} bytes) in {:?}",
                name,
                is_dark,
                svg.len(),
                duration
            );

            // Test HiDPI rasterization at 1.0x, 1.5x, 2.0x scales
            for scale in [1.0f32, 1.5f32, 2.0f32] {
                let r_start = Instant::now();
                let pixmap_res = rasterize_svg_to_pixmap(&svg, scale);
                let r_duration = r_start.elapsed();

                assert!(
                    pixmap_res.is_ok(),
                    "Failed to rasterize '{}' at scale {}: {:?}",
                    name,
                    scale,
                    pixmap_res.err()
                );

                let pixmap = pixmap_res.unwrap();
                assert!(
                    pixmap.width > 0 && pixmap.height > 0,
                    "Zero dimensions rasterized for '{}' at scale {}: {}x{}",
                    name,
                    scale,
                    pixmap.width,
                    pixmap.height
                );

                let expected_bytes = (pixmap.width * pixmap.height * 4) as usize;
                assert_eq!(
                    pixmap.premultiplied_rgba.len(),
                    expected_bytes,
                    "Byte buffer length mismatch for '{}' at scale {}",
                    name,
                    scale
                );

                println!(
                    "  -> Rasterized scale {} ({}x{}) in {:?}",
                    scale, pixmap.width, pixmap.height, r_duration
                );
            }
        }
    }
}

// ============================================================================
// 2. HEAVILY MALFORMED MERMAID STRESS & RESILIENCE (ZERO PANIC / ZERO DEADLOCK)
// ============================================================================

#[test]
fn test_adversarial_malformed_mermaid_resilience_and_graceful_diagnostics() {
    let pathological_inputs = [
        ("empty_string", ""),
        ("whitespace_only", "   \t\n  \r\n   "),
        ("comments_only", "%% this is a comment\n%% another comment"),
        ("random_fuzz_garbage", "akjhf89#$*&^@#$!!~`|}{[]:;\"'\\//..,,??><"),
        (
            "unclosed_square_bracket",
            "flowchart TD\n    A[Unclosed string and bracket",
        ),
        (
            "unclosed_quote_in_sequence",
            "sequenceDiagram\n    Alice->>Bob: \"Unclosed message string\n    Bob-->>Alice: OK",
        ),
        (
            "unclosed_class_brace",
            "classDiagram\n    class BankAccount {\n        +deposit()\n",
        ),
        (
            "unsupported_interface_annotation",
            "classDiagram\n    class FileWatcher {\n        <<interface>>\n        +watch()\n    }",
        ),
        (
            "unclosed_pie_string",
            "pie title Pets\n    \"Dogs : 100\n    \"Cats\" : 50",
        ),
        (
            "cyclic_infinite_chain",
            "flowchart TD\n    A --> B\n    B --> C\n    C --> D\n    D --> E\n    E --> A\n    A --> A",
        ),
        (
            "self_referential_state",
            "stateDiagram-v2\n    [*] --> S1\n    S1 --> S1 : self transition\n    S1 --> [*]",
        ),
        (
            "deeply_nested_subgraphs_20",
            r#"flowchart TB
                subgraph L1 [L1]
                    subgraph L2 [L2]
                        subgraph L3 [L3]
                            subgraph L4 [L4]
                                subgraph L5 [L5]
                                    subgraph L6 [L6]
                                        subgraph L7 [L7]
                                            subgraph L8 [L8]
                                                subgraph L9 [L9]
                                                    subgraph L10 [L10]
                                                        N1 --> N2
                                                    end
                                                end
                                            end
                                        end
                                    end
                                end
                            end
                        end
                    end
                end
            "#,
        ),
        (
            "deeply_nested_sequence_loops",
            r#"sequenceDiagram
                loop Outer 1
                    loop Nested 2
                        loop Nested 3
                            loop Nested 4
                                loop Nested 5
                                    A->>B: Ping
                                    B-->>A: Pong
                                end
                            end
                        end
                    end
                end
            "#,
        ),
        (
            "script_injection_xss_node",
            r#"flowchart TD
                A["<script>alert('xss')</script>"] --> B["<img src='x' onerror='evil()'>"]
                B --> C["javascript:alert(1)"]
            "#,
        ),
        (
            "script_injection_sequence",
            r#"sequenceDiagram
                Alice->>Bob: <iframe src="javascript:evil()">
                Note over Alice,Bob: <svg onload="alert('hack')">
            "#,
        ),
        (
            "unicode_control_and_null_bytes",
            "flowchart TD\n    A[\x01\x02\x03\x04\x05\x06\x07\x08] --> B[\x1b\x1c\x1d\x1e\x1f]",
        ),
        (
            "emoji_and_rtl_arabic_hebrew",
            "flowchart LR\n    😀[مرحبا بالعالم] --> 🚀[שלום עולם] --> 🎉[Complex Mix]",
        ),
        ("missing_header_only_edges", "A --> B\nB --> C"),
        ("header_only_flowchart", "flowchart TD\n"),
        ("header_only_sequence", "sequenceDiagram\n"),
        ("header_only_class", "classDiagram\n"),
        ("header_only_state", "stateDiagram-v2\n"),
        ("header_only_er", "erDiagram\n"),
        ("header_only_pie", "pie\n"),
        ("header_only_gantt", "gantt\n"),
    ];

    for (case_name, input) in pathological_inputs {
        let t_start = Instant::now();

        // Must NEVER panic or deadlock.
        let render_res = render_mermaid_to_svg(input, false);
        let elapsed = t_start.elapsed();

        assert!(
            elapsed.as_millis() < 3000,
            "Pathological case '{}' exceeded 3000 ms timeout (took {:?}) - potential deadlock or exponential complexity!",
            case_name, elapsed
        );

        match render_res {
            Ok(svg) => {
                // If it succeeded, SVG must contain valid svg tags
                assert!(
                    svg.contains("<svg"),
                    "Case '{}' succeeded but output missing <svg tag",
                    case_name
                );

                // When rasterizing, it must either succeed or return a clean RasterError without panicking
                match rasterize_svg_to_pixmap(&svg, 1.0) {
                    Ok(pixmap) => {
                        assert!(pixmap.width > 0 && pixmap.height > 0);
                    }
                    Err(raster_err) => {
                        // Handled by DiagramErrorFallback in the application layer
                        let fallback = DiagramErrorFallback::from_error(
                            input,
                            &MermaidError::RenderError(raster_err.to_string()),
                        );
                        assert!(!fallback.error_message.is_empty());
                    }
                }
            }
            Err(err) => {
                // Verify fallback diagnostics structure
                let fallback = DiagramErrorFallback::from_error(input, &err);
                assert!(
                    !fallback.error_title.is_empty(),
                    "Empty error title for fallback in case '{}'",
                    case_name
                );
                assert!(
                    !fallback.error_message.is_empty(),
                    "Empty error message for fallback in case '{}'",
                    case_name
                );
                assert!(
                    fallback.error_line >= 1,
                    "Invalid error line for fallback in case '{}'",
                    case_name
                );

                if !input.trim().is_empty() {
                    assert!(
                        !fallback.source_lines.is_empty(),
                        "Source lines empty in fallback for case '{}'",
                        case_name
                    );
                }
            }
        }
    }
}

// ============================================================================
// 3. MASSIVE DIAGRAM PERFORMANCE & BOUNDED MEMORY STRESS
// ============================================================================

#[test]
fn test_adversarial_massive_mermaid_graph() {
    // Generate a 100-node, 100-edge flowchart
    let mut large_graph = String::from("flowchart TD\n");
    for i in 0..100 {
        large_graph.push_str(&format!("    Node{:03}[\"Item {}\"] --> Node{:03}[\"Item {}\"]\n", i, i, (i + 1) % 100, (i + 1) % 100));
    }

    let t_start = Instant::now();
    let res = render_mermaid_to_svg(&large_graph, true);
    let elapsed = t_start.elapsed();

    assert!(
        res.is_ok(),
        "Massive 100-node graph failed to render: {:?}",
        res.err()
    );
    assert!(
        elapsed.as_millis() < 5000,
        "Massive 100-node graph took {:?}, exceeding 5000 ms target",
        elapsed
    );

    let svg = res.unwrap();
    let raster_res = rasterize_svg_to_pixmap(&svg, 1.0);
    assert!(raster_res.is_ok(), "Failed to rasterize 100-node graph pixmap");
}

// ============================================================================
// 4. PATH SAFETY & TRAVERSAL ADVERSARIAL STRESS (CRATES/CORE)
// ============================================================================

#[test]
fn test_adversarial_windows_reserved_names_exhaustive() {
    let base_reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    for name in base_reserved {
        // 1. Exact uppercase, lowercase, mixed case
        for variant in [name.to_uppercase(), name.to_lowercase(), format!("{}{}", &name[..1].to_uppercase(), &name[1..].to_lowercase())] {
            assert!(
                matches!(validate_filename(&variant), Err(PathSafetyError::ReservedName(_))),
                "Failed to reject reserved name '{}'", variant
            );

            // 2. With common markdown extensions (.md, .markdown, .txt, .json)
            for ext in [".md", ".markdown", ".txt", ".json", ".log"] {
                let with_ext = format!("{}{}", variant, ext);
                assert!(
                    matches!(validate_filename(&with_ext), Err(PathSafetyError::ReservedName(_))),
                    "Failed to reject reserved name with extension '{}'", with_ext
                );
            }
        }
    }
}

#[test]
fn test_adversarial_directory_traversal_variations() {
    let vault = tempdir().expect("tempdir");
    let vault_root = vault.path();

    let malicious_traversal_paths = [
        "../escaped.md",
        "../../etc/passwd",
        r"..\..\windows\system32\calc.exe",
        "sub/../../escaped.md",
        "sub/nested/../../../outside.md",
        "./../../escaped.md",
        "folder/../..",
        r"..\outside.md",
        r"sub\..\..\escape.txt",
    ];

    for malicious in malicious_traversal_paths {
        let res = sanitize_path(vault_root, Path::new(malicious));
        assert!(
            matches!(res, Err(PathSafetyError::TraversalAttempted(_))),
            "Failed to reject directory traversal for '{}': {:?}",
            malicious,
            res
        );
    }
}

#[test]
fn test_adversarial_absolute_paths_outside_vault() {
    let vault = tempdir().expect("tempdir");
    let vault_root = vault.path();

    let outside_absolute_paths = [
        r"C:\Windows\System32\notepad.exe",
        r"C:\Users\Administrator\Desktop\secret.txt",
        r"D:\other_drive\vault\note.md",
        r"\root_relative\note.md",
        "/etc/shadow",
        "/var/log/syslog",
    ];

    for abs_path in outside_absolute_paths {
        let res = sanitize_path(vault_root, Path::new(abs_path));
        assert!(
            res.is_err(),
            "Absolute path outside vault was unexpectedly accepted: '{}'",
            abs_path
        );
    }
}

#[test]
fn test_adversarial_illegal_characters_and_control_chars() {
    let illegal_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

    for ch in illegal_chars {
        let filename = format!("note{}test.md", ch);
        let res = validate_filename(&filename);
        assert!(
            matches!(res, Err(PathSafetyError::IllegalCharacters(_))),
            "Failed to reject illegal char '{}' in filename '{}'",
            ch, filename
        );
    }

    // ASCII control characters (0..=31)
    for code in 0..=31u8 {
        let ch = code as char;
        let filename = format!("test{}file.md", ch);
        let res = validate_filename(&filename);
        assert!(
            matches!(res, Err(PathSafetyError::IllegalCharacters(_))),
            "Failed to reject ASCII control char {} (code {})",
            ch.escape_default(), code
        );
    }
}

#[test]
fn test_adversarial_trailing_periods_and_spaces() {
    let bad_trailing = [
        "note.",
        "note ",
        "note.md.",
        "note.md ",
        "note. ",
        "note.  ",
        "   ",
        ".",
        "..",
    ];

    for name in bad_trailing {
        let res = validate_filename(name);
        assert!(
            res.is_err(),
            "Filename with invalid trailing chars or traversal '{}' was unexpectedly accepted",
            name
        );
    }
}

#[test]
fn test_adversarial_valid_cross_platform_filenames_accepted() {
    let valid_names = [
        "Simple Note.md",
        "note_with_underscores-and-hyphens.markdown",
        "v1.2.3.final.md",
        "Architecture (AirGap) 2026.md",
        "Deeply Nested Note [Official].md",
        "日本語のメモ.md",
        "中文笔记.markdown",
        "한국어_노트.md",
        "Notes & Ideas + Plans.md",
        "50% discount #special @vault.md",
    ];

    for name in valid_names {
        assert!(
            validate_filename(name).is_ok(),
            "Valid filename '{}' was falsely rejected: {:?}",
            name,
            validate_filename(name).err()
        );
    }
}
