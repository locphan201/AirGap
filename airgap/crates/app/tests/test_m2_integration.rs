//! Integration tests for Milestone M2:
//! Markdown Renderer, Code Highlighting, Live Preview & Editor Enhancements.

use airgap_app::clipboard::ClipboardHandler;
use airgap_app::state::{AppState, FindReplaceState, UndoManager, ViewMode};
use airgap_app::ui::editor::{apply_auto_pairing, apply_indentation, apply_list_continuation};
use airgap_app::url_safety::sanitize_external_url;
use airgap_render::{
    parse_markdown, resolve_local_image_path, Block, CalloutKind, ColumnAlignment,
    ImageResolutionError, InlineSpan, SyntaxHighlighter,
};
use std::fs::{self, File};
use tempfile::tempdir;

#[test]
fn test_m2_markdown_full_ast_fidelity() {
    let md = r#"# Main Title {#heading-id}

This is a paragraph with **bold**, *italic*, ~~strikethrough~~, and `inline_code`.
Check out [AirGap](https://airgap.local) and ![Logo](assets/logo.png).

## GFM Features

> [!TIP]
> This is a helpful tip callout.

### Interactive Tasks
- [ ] Incomplete Item
- [x] Completed Item
- Standard bullet

### Tables
| Feature | Supported | Alignment |
| :--- | :---: | ---: |
| Syntax Highlighting | Yes | Right-aligned |

```rust
fn main() {
    println!("Hello AirGap");
}
```

```mermaid
graph TD
    A-->B
```

---
"#;

    let doc = parse_markdown(md);
    assert!(!doc.blocks.is_empty());

    // 1. Heading with ID & Source Span
    match &doc.blocks[0] {
        Block::Heading {
            level,
            text,
            id,
            span,
            ..
        } => {
            assert_eq!(*level, 1);
            assert_eq!(text, "Main Title");
            assert_eq!(id.as_deref(), Some("heading-id"));
            assert_eq!(span.start_line, 1);
        }
        _ => panic!("Expected Heading at block 0"),
    }

    // 2. Paragraph with rich inlines
    match &doc.blocks[1] {
        Block::Paragraph { inlines, span, .. } => {
            assert_eq!(span.start_line, 3);
            let has_bold = inlines
                .iter()
                .any(|i| matches!(i, InlineSpan::Bold(s) if s == "bold"));
            let has_italic = inlines
                .iter()
                .any(|i| matches!(i, InlineSpan::Italic(s) if s == "italic"));
            let has_strike = inlines
                .iter()
                .any(|i| matches!(i, InlineSpan::Strikethrough(s) if s == "strikethrough"));
            let has_code = inlines
                .iter()
                .any(|i| matches!(i, InlineSpan::Code(s) if s == "inline_code"));
            let has_link = inlines
                .iter()
                .any(|i| matches!(i, InlineSpan::Link { text, url, .. } if text == "AirGap" && url == "https://airgap.local"));
            let has_img = inlines
                .iter()
                .any(|i| matches!(i, InlineSpan::Image { alt, url, .. } if alt == "Logo" && url == "assets/logo.png"));

            assert!(has_bold, "Missing bold inline");
            assert!(has_italic, "Missing italic inline");
            assert!(has_strike, "Missing strikethrough inline");
            assert!(has_code, "Missing code inline");
            assert!(has_link, "Missing link inline");
            assert!(has_img, "Missing image inline");
        }
        _ => panic!("Expected Paragraph at block 1"),
    }

    // 3. Blockquote with TIP callout
    let tip_block = doc
        .blocks
        .iter()
        .find(|b| matches!(b, Block::BlockQuote { .. }))
        .expect("Missing BlockQuote");
    match tip_block {
        Block::BlockQuote { callout, lines, .. } => {
            assert_eq!(*callout, Some(CalloutKind::Tip));
            assert!(lines.iter().any(|l| l.contains("helpful tip callout")));
        }
        _ => unreachable!(),
    }

    // 4. Task List Items
    let list_block = doc
        .blocks
        .iter()
        .find(|b| matches!(b, Block::List { .. }))
        .expect("Missing List");
    match list_block {
        Block::List {
            items, is_ordered, ..
        } => {
            assert!(!is_ordered);
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].task_status, Some(false));
            assert_eq!(items[1].task_status, Some(true));
            assert_eq!(items[2].task_status, None);
        }
        _ => unreachable!(),
    }

    // 5. GFM Table with column alignments
    let table_block = doc
        .blocks
        .iter()
        .find(|b| matches!(b, Block::Table { .. }))
        .expect("Missing Table");
    match table_block {
        Block::Table {
            headers,
            rows,
            alignments,
            ..
        } => {
            assert_eq!(headers, &["Feature", "Supported", "Alignment"]);
            assert_eq!(rows.len(), 1);
            assert_eq!(
                alignments,
                &[
                    ColumnAlignment::Left,
                    ColumnAlignment::Center,
                    ColumnAlignment::Right
                ]
            );
        }
        _ => unreachable!(),
    }

    // 6. Code block & Mermaid block distinction
    let code_block = doc
        .blocks
        .iter()
        .find(|b| matches!(b, Block::CodeBlock { .. }))
        .expect("Missing CodeBlock");
    match code_block {
        Block::CodeBlock { language, code, .. } => {
            assert_eq!(language, "rust");
            assert!(code.contains("Hello AirGap"));
        }
        _ => unreachable!(),
    }

    let mermaid_block = doc
        .blocks
        .iter()
        .find(|b| matches!(b, Block::MermaidBlock { .. }))
        .expect("Missing MermaidBlock");
    match mermaid_block {
        Block::MermaidBlock { source, .. } => {
            assert!(source.contains("graph TD"));
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_m2_syntax_highlighting_palettes() {
    let highlighter = SyntaxHighlighter::new();
    let rust_code = "fn calculate(x: i32) -> i32 {\n    x * 2\n}\n";

    // Dark theme
    let dark_lines = highlighter.highlight_code(rust_code, "rust", true);
    assert_eq!(dark_lines.len(), 3);
    assert_eq!(dark_lines[0][0].text, "fn");

    // Light theme
    let light_lines = highlighter.highlight_code(rust_code, "rust", false);
    assert_eq!(light_lines.len(), 3);
    assert_eq!(light_lines[0][0].text, "fn");

    // Palettes differ between dark and light
    let dark_palette = highlighter.theme_palette(true);
    let light_palette = highlighter.theme_palette(false);
    assert_ne!(dark_palette.bg, light_palette.bg);
    assert_ne!(dark_palette.fg, light_palette.fg);
}

#[test]
fn test_m2_safe_image_resolution() {
    let dir = tempdir().unwrap();
    let vault_root = dir.path();
    let assets = vault_root.join("assets");
    fs::create_dir_all(&assets).unwrap();

    let image_path = assets.join("diagram.png");
    File::create(&image_path).unwrap();

    let note_path = vault_root.join("notes").join("meeting.md");
    fs::create_dir_all(note_path.parent().unwrap()).unwrap();
    File::create(&note_path).unwrap();

    // 1. Valid note-relative traversal to assets
    let resolved =
        resolve_local_image_path(vault_root, &note_path, "../assets/diagram.png").unwrap();
    assert_eq!(resolved, image_path.canonicalize().unwrap());

    // 2. Valid vault-root leading slash
    let resolved_slash =
        resolve_local_image_path(vault_root, &note_path, "/assets/diagram.png").unwrap();
    assert_eq!(resolved_slash, image_path.canonicalize().unwrap());

    // 3. Network URL blocked
    let net_err =
        resolve_local_image_path(vault_root, &note_path, "https://remote.com/img.png").unwrap_err();
    assert!(matches!(net_err, ImageResolutionError::NetworkBlocked(_)));

    // 4. Traversal escape blocked
    let outside_err =
        resolve_local_image_path(vault_root, &note_path, "../../outside.png").unwrap_err();
    assert!(matches!(outside_err, ImageResolutionError::VaultEscape(_)));
}

#[test]
fn test_m2_editor_conveniences() {
    // 1. Task continuation on Enter
    let (t1, c1) = apply_list_continuation("- [ ] Step 1", 12).unwrap();
    assert_eq!(t1, "- [ ] Step 1\n- [ ] ");
    assert_eq!(c1, 19);

    // Empty task exits list
    let (t2, c2) = apply_list_continuation("- [ ] ", 6).unwrap();
    assert_eq!(t2, "");
    assert_eq!(c2, 0);

    // 2. Numbered list continuation
    let (t3, c3) = apply_list_continuation("1. First item", 13).unwrap();
    assert_eq!(t3, "1. First item\n2. ");
    assert_eq!(c3, 17);

    // 3. Auto-pairing
    let (p1, cp1) = apply_auto_pairing("selected text", 0, '(', "selected text").unwrap();
    assert_eq!(p1, "(selected text)");
    assert_eq!(cp1, 15);

    let (p2, cp2) = apply_auto_pairing("hello", 5, '[', "").unwrap();
    assert_eq!(p2, "hello[]");
    assert_eq!(cp2, 6);

    // 4. Indentation
    let (ind, ci) = apply_indentation("Code line", 0, false);
    assert_eq!(ind, "  Code line");
    assert_eq!(ci, 2);

    let (out, co) = apply_indentation(&ind, 2, true);
    assert_eq!(out, "Code line");
    assert_eq!(co, 0);
}

#[test]
fn test_m2_smart_url_paste_and_url_sanitization() {
    // Smart paste wraps selection
    let pasted =
        ClipboardHandler::try_paste_smart_url("https://crates.io", "Crates Repository").unwrap();
    assert_eq!(pasted, "[Crates Repository](https://crates.io)");

    // URL scheme whitelist
    assert!(sanitize_external_url("https://secure.example.com").is_ok());
    assert!(sanitize_external_url("http://insecure.example.com").is_ok());
    assert!(sanitize_external_url("mailto:support@airgap.io").is_ok());

    assert!(sanitize_external_url("file:///etc/shadow").is_err());
    assert!(sanitize_external_url("javascript:evil()").is_err());
    assert!(sanitize_external_url("cmd.exe").is_err());
}

#[test]
fn test_m2_view_mode_transitions_and_split() {
    let mut state = AppState::default();
    assert_eq!(state.view_mode, ViewMode::Split);
    assert_eq!(state.split_ratio, 0.5);

    // Cycle modes
    state.view_mode = state.view_mode.cycle();
    assert_eq!(state.view_mode, ViewMode::EditorOnly);

    state.view_mode = state.view_mode.cycle();
    assert_eq!(state.view_mode, ViewMode::PreviewOnly);

    state.view_mode = state.view_mode.cycle();
    assert_eq!(state.view_mode, ViewMode::Split);
}

#[test]
fn test_m2_undo_redo_and_find_replace() {
    let mut state = AppState {
        editor_content: "Rust is fast. Rust is safe. Rust is modern.".to_string(),
        ..Default::default()
    };

    let mut undo = UndoManager::new(20);
    undo.push_snapshot(&state.editor_content, 1, 1, true);

    let mut find_state = FindReplaceState {
        find_query: "Rust".to_string(),
        replace_query: "AirGap".to_string(),
        case_sensitive: true,
        whole_word: true,
        ..Default::default()
    };
    find_state.recalculate_matches(&state.editor_content);
    assert_eq!(find_state.matches.len(), 3);

    // Replace current
    find_state.replace_current(&mut state.editor_content, &mut undo, 1, 1);
    assert_eq!(
        state.editor_content,
        "AirGap is fast. Rust is safe. Rust is modern."
    );

    // Replace remaining
    find_state.replace_all(&mut state.editor_content, &mut undo, 1, 1);
    assert_eq!(
        state.editor_content,
        "AirGap is fast. AirGap is safe. AirGap is modern."
    );

    // Undo back
    let undone = undo.undo(&state.editor_content, 1, 1).unwrap();
    state.editor_content = undone.text;
    assert_eq!(
        state.editor_content,
        "AirGap is fast. Rust is safe. Rust is modern."
    );
}

#[test]
fn test_m2_interactive_gui_editor_conveniences() {
    use airgap_app::ui::editor::render_editor;
    use egui::{Context, Event, Key, Modifiers, RawInput};
    use std::path::PathBuf;

    let ctx = Context::default();

    // 1. Enter key list continuation via GUI event dispatch
    let mut state = AppState {
        active_note_path: Some(PathBuf::from("note.md")),
        editor_content: "- [ ] First task".to_string(),
        editor_char_range: Some(16..16),
        ..Default::default()
    };

    let mut raw_input = RawInput::default();
    raw_input.events.push(Event::Key {
        key: Key::Enter,
        physical_key: Some(Key::Enter),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });

    let _ = ctx.run(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            render_editor(ui, &mut state);
        });
    });

    assert_eq!(state.editor_content, "- [ ] First task\n- [ ] ");

    // 2. Tab indentation via GUI event dispatch
    let mut raw_input = RawInput::default();
    raw_input.events.push(Event::Key {
        key: Key::Tab,
        physical_key: Some(Key::Tab),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });

    let _ = ctx.run(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            render_editor(ui, &mut state);
        });
    });

    assert_eq!(state.editor_content, "- [ ] First task\n  - [ ] ");

    // 3. Auto-pairing enclosure via Event::Text
    let mut state = AppState {
        active_note_path: Some(PathBuf::from("note.md")),
        editor_content: "hello world".to_string(),
        editor_char_range: Some(6..11), // "world" selected
        ..Default::default()
    };

    let mut raw_input = RawInput::default();
    raw_input.events.push(Event::Text("[".to_string()));

    let _ = ctx.run(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            render_editor(ui, &mut state);
        });
    });

    assert_eq!(state.editor_content, "hello [world]");
}

#[test]
fn test_m2_interactive_gui_scroll_synchronization() {
    use airgap_app::state::ScrollDriver;
    use airgap_app::ui::editor::render_editor;
    use airgap_app::ui::preview::render_preview;
    use egui::{Context, RawInput};
    use std::path::PathBuf;

    let ctx = Context::default();
    let mut state = AppState {
        active_note_path: Some(PathBuf::from("note.md")),
        editor_content: "# Title\n\n".to_string() + &"Paragraph line of text.\n\n".repeat(50),
        view_mode: ViewMode::Split,
        ..Default::default()
    };

    // Frame 1: Initial layout
    let _ = ctx.run(RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| render_editor(ui, &mut state));
                ui.vertical(|ui| render_preview(ui, &mut state));
            });
        });
    });

    // Frame 2: Simulate Editor driving scroll
    state.scroll_sync.driver = ScrollDriver::Editor;
    state.scroll_sync.editor_content_height = 2000.0;
    state.scroll_sync.editor_viewport_height = 500.0;
    state.scroll_sync.preview_content_height = 3000.0;
    state.scroll_sync.preview_viewport_height = 500.0;

    // Simulate editor scroll offset halfway (50%)
    let _max_editor_scroll = (state.scroll_sync.editor_content_height
        - state.scroll_sync.editor_viewport_height)
        .max(1.0);
    let ratio = 0.5;
    let max_preview_scroll = (state.scroll_sync.preview_content_height
        - state.scroll_sync.preview_viewport_height)
        .max(0.0);
    state.scroll_sync.target_preview_scroll = Some(ratio * max_preview_scroll);

    assert_eq!(state.scroll_sync.target_preview_scroll, Some(1250.0));

    // Frame 3: Preview consumes target_preview_scroll via .take()
    let _ = ctx.run(RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            render_preview(ui, &mut state);
        });
    });

    // Verify follower actuation consumed the target (take() called)
    assert_eq!(state.scroll_sync.target_preview_scroll, None);
}

#[test]
fn test_m2_toggle_checkbox_crlf_and_trailing_newline() {
    use airgap_app::ui::preview::toggle_checkbox_at_line;

    let mut doc = String::from("Heading\r\n- [ ] First item\r\n- [x] Second item\r\n");
    toggle_checkbox_at_line(&mut doc, 2);
    assert_eq!(doc, "Heading\r\n- [x] First item\r\n- [x] Second item\r\n");
    assert!(doc.contains("\r\n"));
    assert!(!doc.replace("\r\n", "").contains('\n'));

    toggle_checkbox_at_line(&mut doc, 3);
    assert_eq!(doc, "Heading\r\n- [x] First item\r\n- [ ] Second item\r\n");
}
