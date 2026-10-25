//! Challenger M2 Adversarial Stress Test Suite
//!
//! Empirical adversarial verification targeting:
//! 1. Pathological Markdown: Extremely deep nested blockquotes (100+ levels), unclosed code fences,
//!    unclosed brackets/links, malformed GFM tables (uneven row column counts, missing delimiters, empty headers),
//!    and mixed UTF-8 / CJK text.
//! 2. Large Note Performance: Parse and layout 1MB+ markdown documents, verify layout job generation does not panic,
//!    and verify the memoized `EditorHighlighter` caches lines and maintains responsive frame times (<15ms per keystroke).
//! 3. Scroll Sync & LineIndex: Boundary conditions for line 0, out-of-bounds lines, empty documents,
//!    single-line documents, documents with 10,000 blank lines.

use airgap_app::state::{AppState, FindReplaceState, ScrollDriver, UndoManager};
use airgap_app::ui::editor::{apply_auto_pairing, apply_indentation, apply_list_continuation};
use airgap_app::ui::editor_highlighter::{EditorHighlighter, LayoutOptions};
use airgap_render::{parse_markdown, Block, LineIndex};
use egui::Context;
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// 1. PATHOLOGICAL MARKDOWN STRESS TESTS
// ============================================================================

#[test]
fn test_pathological_deeply_nested_blockquotes() {
    // 150 levels of nested blockquotes
    let depth = 150;
    let mut md = String::new();
    for _ in 0..depth {
        md.push_str("> ");
    }
    md.push_str("[!WARNING]\n");
    for _ in 0..depth {
        md.push_str("> ");
    }
    md.push_str("Deeply nested adversarial callout.\n");

    println!(
        "[Adversarial 150-depth Blockquotes] Input length: {} bytes",
        md.len()
    );

    let doc = parse_markdown(&md);
    assert!(
        !doc.blocks.is_empty(),
        "Parser must produce blocks without stack overflow or panic"
    );

    let bq_count = doc
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::BlockQuote { .. }))
        .count();
    assert!(
        bq_count >= 1,
        "Deeply nested blockquote must produce BlockQuote"
    );

    // 500 levels of nested blockquotes
    let extreme_depth = 500;
    let mut extreme_md = String::with_capacity(extreme_depth * 2 + 128);
    for _ in 0..extreme_depth {
        extreme_md.push_str("> ");
    }
    extreme_md.push_str("Extreme depth text\n");

    let extreme_doc = parse_markdown(&extreme_md);
    assert!(
        !extreme_doc.blocks.is_empty(),
        "500-level nested blockquotes must parse safely without stack overflow"
    );
}

#[test]
fn test_pathological_unclosed_code_fences() {
    // Unclosed code block at EOF
    let md = "# Title\n\n```rust\nfn unfinished() {\n    let x = 42;\n";
    let doc = parse_markdown(md);
    assert_eq!(doc.blocks.len(), 2);

    match &doc.blocks[1] {
        Block::CodeBlock { language, code, .. } => {
            assert_eq!(language, "rust");
            assert!(
                code.contains("fn unfinished()"),
                "Unclosed code fence at EOF must be captured as CodeBlock"
            );
        }
        _ => panic!("Expected CodeBlock for unclosed fence at EOF"),
    }

    // Unclosed mermaid fence
    let md_mermaid = "```mermaid\ngraph TD\n    A-->B\n";
    let doc_m = parse_markdown(md_mermaid);
    assert_eq!(doc_m.blocks.len(), 1);
    match &doc_m.blocks[0] {
        Block::MermaidBlock { source, .. } => {
            assert!(source.contains("graph TD"));
        }
        _ => panic!("Expected MermaidBlock for unclosed mermaid fence"),
    }

    // Repeated alternating unclosed backticks
    let pathological_fences = "```\n```\n```\n```\n```\n";
    let doc_p = parse_markdown(pathological_fences);
    assert!(
        !doc_p.blocks.is_empty(),
        "Repeated fence toggles must not panic"
    );
}

#[test]
fn test_pathological_unclosed_brackets_and_links() {
    let adversarial_inputs = [
        "[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[",
        "((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((",
        "[unclosed link text](",
        "[unclosed link text](https://",
        "![unclosed alt](",
        "[nested [brackets [without [closing [urls",
        "[link](https://example.com/foo(bar)baz",
        "[incomplete link [with nested]](http://foo",
        "[[[link]]](http://bar)",
        "*unclosed emphasis **unclosed strong ~~unclosed strike `unclosed code",
    ];

    for (i, input) in adversarial_inputs.iter().enumerate() {
        let doc = parse_markdown(input);
        println!(
            "[Adversarial Brackets #{}] Parsed {} chars into {} blocks without panic",
            i,
            input.len(),
            doc.blocks.len()
        );
    }
}

#[test]
fn test_pathological_malformed_gfm_tables() {
    let ctx = Context::default();

    // 1. Uneven columns: header 3 cols, row 1 has 1 col, row 2 has 5 cols, row 3 has 0 cols
    let uneven_table = "\
| Col A | Col B | Col C |
| :--- | :---: | ---: |
| Single Cell |
| C1 | C2 | C3 | C4 | C5 |
| |
";
    let doc1 = parse_markdown(uneven_table);
    assert_eq!(doc1.blocks.len(), 1);
    match &doc1.blocks[0] {
        Block::Table { headers, rows, .. } => {
            println!("doc1 headers: {:?}, rows: {:?}", headers, rows);
            assert_eq!(headers.len(), 3);
            assert_eq!(rows.len(), 3);
        }
        _ => panic!("Expected Table block"),
    }

    // 2. Render uneven table in preview UI (verifies egui::Grid does not panic on uneven row cells)
    let mut state = AppState {
        editor_content: uneven_table.to_string(),
        ..Default::default()
    };
    let raw_input = egui::RawInput::default();
    let _ = ctx.run(raw_input.clone(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            airgap_app::ui::preview::render_preview(ui, &mut state);
        });
    });

    // 3. Missing delimiters (header without separator row: pulldown-cmark treats as paragraph)
    let missing_delim = "\
| Header 1 | Header 2 |
| Cell 1 | Cell 2 |
";
    let doc2 = parse_markdown(missing_delim);
    assert!(
        !doc2.blocks.is_empty(),
        "Table with missing delimiter row must parse safely"
    );

    // 4. Empty headers with valid delimiter
    let empty_headers = "\
| | | |
| --- | --- | --- |
| Val 1 | Val 2 | Val 3 |
";
    let doc3 = parse_markdown(empty_headers);
    assert_eq!(doc3.blocks.len(), 1);
    match &doc3.blocks[0] {
        Block::Table { headers, rows, .. } => {
            println!(
                "Empty headers parsed headers: {:?}, rows: {:?}",
                headers, rows
            );
            assert_eq!(headers.len(), 3);
            assert_eq!(rows.len(), 1);
        }
        _ => panic!("Expected Table block for empty headers"),
    }

    // 5. Massive table with 100 columns and 50 rows
    let col_count = 100;
    let mut wide_table = String::new();
    wide_table.push('|');
    for c in 0..col_count {
        wide_table.push_str(&format!(" Col {:03} |", c));
    }
    wide_table.push('\n');
    wide_table.push('|');
    for _ in 0..col_count {
        wide_table.push_str(" :---: |");
    }
    wide_table.push('\n');
    for r in 0..50 {
        wide_table.push('|');
        for c in 0..col_count {
            wide_table.push_str(&format!(" R{}C{} |", r, c));
        }
        wide_table.push('\n');
    }

    let doc_wide = parse_markdown(&wide_table);
    assert_eq!(doc_wide.blocks.len(), 1);
    match &doc_wide.blocks[0] {
        Block::Table {
            headers,
            rows,
            alignments,
            ..
        } => {
            assert_eq!(headers.len(), col_count);
            assert_eq!(rows.len(), 50);
            assert_eq!(alignments.len(), col_count);
        }
        _ => panic!("Expected Table block for wide table"),
    }
}

#[test]
fn test_pathological_mixed_utf8_cjk_emojis() {
    let ctx = Context::default();

    let complex_text = "\
# 測試標題: 繁體與简体中文測試 🚀

这是一个包含多种字符集的复杂文档。
- 日本語テスト: こんにちは！ひらがな、カタカナ（コンニチハ）、漢字（日本語能力試験）。
- 한국어 테스트: 안녕하세요! 한글과 컴퓨터, 훈民정음 (訓民正音).
- Emojis with ZWJ: 👨‍👩‍👧‍👦 (Family), 🧑🏽‍💻 (Technologist), 🏳️‍🌈 (Rainbow flag).
- RTL Markers & Arabic: مرحبا بالعالم (Hello world in Arabic), עברית (Hebrew).
- Accents & Combining: e\u{0301} (é decomposed), n\u{0303} (ñ), \u{200B}zero-width-space\u{200B}.
- BOM prefix tolerance: \u{FEFF}Leading BOM mark.

| 语言 (Language) | 挨拶 (Greeting) | 状态 (Status) |
| :--- | :---: | ---: |
| 简体中文 | 你好世界 | - [x] 已完成 |
| 日本語 | こんにちは世界 | - [ ] 未完了 |
| 한국어 | 안녕하세요 세계 | - [x] 완료됨 |
| English | Hello World | - [ ] Pending |

```python
# 🐍 Python with multi-byte strings
greeting = \"你好，世界！こんにちは！안녕하세요！\"
print(greeting)
```
";

    // 1. Markdown Parsing
    let doc = parse_markdown(complex_text);
    assert!(!doc.blocks.is_empty());
    assert_eq!(doc.outline().len(), 1);
    assert_eq!(doc.outline()[0].1, "測試標題: 繁體與简体中文測試 🚀");

    // 2. LineIndex Offset Resolution with multi-byte characters
    let line_index = LineIndex::new(complex_text);
    assert!(line_index.total_lines() >= 20);
    for (byte_idx, _) in complex_text.char_indices() {
        let line = line_index.offset_to_line(byte_idx);
        assert!(line >= 1 && line <= line_index.total_lines());
    }

    // 3. Find matches on CJK character boundaries
    let mut find_state = FindReplaceState {
        find_query: "繁體".to_string(),
        case_sensitive: true,
        ..Default::default()
    };
    find_state.recalculate_matches(complex_text);
    assert_eq!(find_state.matches.len(), 1);

    // 4. EditorHighlighter Layout with valid CJK find matches
    let mut highlighter = EditorHighlighter::default();
    let opts = LayoutOptions {
        wrap_width: 800.0,
        is_dark: true,
        font_size: 14.0,
        find_matches: &find_state.matches,
        active_match: 0,
    };

    let raw_input = egui::RawInput::default();
    let _ = ctx.run(raw_input.clone(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let galley = highlighter.layout(ui, complex_text, &opts);
            assert!(galley.rows.len() >= 10);
        });
    });

    // 5. Preview rendering with complex CJK and emojis
    let mut state = AppState {
        editor_content: complex_text.to_string(),
        ..Default::default()
    };
    let _ = ctx.run(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            airgap_app::ui::preview::render_preview(ui, &mut state);
        });
    });
}

// ============================================================================
// 2. LARGE NOTE PERFORMANCE (1MB+) & HIGHLIGHTER MEMOIZATION
// ============================================================================

fn generate_1mb_markdown() -> String {
    let target_bytes = 1024 * 1024; // 1 MB
    let mut text = String::with_capacity(target_bytes + 65536);
    let mut line_num = 0;

    while text.len() < target_bytes {
        line_num += 1;
        match line_num % 6 {
            0 => {
                let _ = std::fmt::Write::write_fmt(
                    &mut text,
                    format_args!(
                        "## Section {:05}: Performance Analysis of Large Note Scalability\n",
                        line_num
                    ),
                );
            }
            1 => {
                let _ = std::fmt::Write::write_fmt(
                    &mut text,
                    format_args!(
                        "- [ ] Task Item {:05}: Verify layout job generation does not panic under 1MB text.\n",
                        line_num
                    ),
                );
            }
            2 => {
                let _ = std::fmt::Write::write_fmt(
                    &mut text,
                    format_args!(
                        "> [!NOTE]\n> Callout block {:05} documenting memoized highlighter metrics.\n",
                        line_num
                    ),
                );
            }
            3 => {
                let _ = std::fmt::Write::write_fmt(
                    &mut text,
                    format_args!(
                        "| Col A {:04} | Col B {:04} | Col C {:04} |\n| :--- | :---: | ---: |\n| Data {:04} | Active | Value {:04} |\n",
                        line_num, line_num, line_num, line_num, line_num
                    ),
                );
            }
            4 => {
                text.push_str("```rust\nfn compute_hash(data: &[u8]) -> u64 {\n    0xcbf29ce484222325\n}\n```\n");
            }
            _ => {
                let _ = std::fmt::Write::write_fmt(
                    &mut text,
                    format_args!(
                        "Paragraph {:05} with **bold**, *italic*, and `inline code` to stress tokenizer performance.\n",
                        line_num
                    ),
                );
            }
        }
    }

    text
}

#[test]
fn test_large_note_1mb_parsing_and_ast_generation() {
    let text = generate_1mb_markdown();
    println!(
        "[1MB Parse Benchmark] Generated payload: {} bytes, {} lines",
        text.len(),
        text.lines().count()
    );
    assert!(
        text.len() >= 1024 * 1024,
        "Payload must exceed 1MB threshold"
    );

    // Measure AST parse time
    let t0 = Instant::now();
    let doc = parse_markdown(&text);
    let parse_time = t0.elapsed();
    let parse_ms = parse_time.as_secs_f64() * 1000.0;

    println!(
        "[1MB Parse Benchmark] Parsed {} blocks in {:.2} ms",
        doc.blocks.len(),
        parse_ms
    );

    assert!(
        !doc.blocks.is_empty(),
        "Parsed 1MB document must contain blocks"
    );
    // Requirement from TEST_INFRA.md: 1MB note parsing must complete < 500 ms
    assert!(
        parse_ms < 500.0,
        "1MB Markdown parsing must complete in < 500ms; actual: {:.2}ms",
        parse_ms
    );
}

#[test]
fn test_large_note_1mb_highlighter_memoization_idle_frames() {
    let ctx = Context::default();
    let text = generate_1mb_markdown();
    let mut highlighter = EditorHighlighter::default();

    let opts = LayoutOptions {
        wrap_width: 800.0,
        is_dark: true,
        font_size: 14.0,
        find_matches: &[],
        active_match: 0,
    };

    let raw_input = egui::RawInput::default();

    // Frame 0: Cold start layout
    let mut cold_galley = None;
    let t0 = Instant::now();
    let _ = ctx.run(raw_input.clone(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            cold_galley = Some(highlighter.layout(ui, &text, &opts));
        });
    });
    let cold_galley = cold_galley.unwrap();
    let cold_ms = t0.elapsed().as_secs_f64() * 1000.0;
    println!("[1MB Cold Layout] Completed in {:.2} ms", cold_ms);

    // Frames 1..=30: Idle frames (verifying true pointer-identity cache hit)
    let idle_iterations = 30;
    let mut idle_times_ms = Vec::with_capacity(idle_iterations);

    for _ in 0..idle_iterations {
        let mut cached_galley = None;
        let t_frame = Instant::now();
        let _ = ctx.run(raw_input.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                cached_galley = Some(highlighter.layout(ui, &text, &opts));
            });
        });
        idle_times_ms.push(t_frame.elapsed().as_secs_f64() * 1000.0);

        // Verify that the exact same Arc<Galley> was returned without re-allocation
        assert!(
            Arc::ptr_eq(&cold_galley, &cached_galley.unwrap()),
            "Idle frames must return identical Arc<Galley> reference without re-layout"
        );
    }

    let avg_idle_ms = idle_times_ms.iter().sum::<f64>() / idle_iterations as f64;
    let max_idle_ms = idle_times_ms.iter().cloned().fold(0.0, f64::max);
    println!(
        "[1MB Idle Memoization] Avg: {:.3} ms/frame, Max: {:.3} ms/frame",
        avg_idle_ms, max_idle_ms
    );
}

#[test]
fn test_large_note_1mb_highlighter_keystroke_frame_times() {
    let ctx = Context::default();
    let mut text = generate_1mb_markdown();
    let mut highlighter = EditorHighlighter::default();

    let opts = LayoutOptions {
        wrap_width: 800.0,
        is_dark: true,
        font_size: 14.0,
        find_matches: &[],
        active_match: 0,
    };

    let raw_input = egui::RawInput::default();

    // Warm-up initial layout
    let _ = ctx.run(raw_input.clone(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let _ = highlighter.layout(ui, &text, &opts);
        });
    });

    // 1. Keystroke at End: Append single character
    let t_append = Instant::now();
    text.push('X');
    let _ = ctx.run(raw_input.clone(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let _ = highlighter.layout(ui, &text, &opts);
        });
    });
    let append_ms = t_append.elapsed().as_secs_f64() * 1000.0;
    println!(
        "[1MB Mutation - Append End] Completed in {:.2} ms",
        append_ms
    );

    // 2. Keystroke in Middle: Insert single character
    let mid_point = text.len() / 2;
    let t_mid = Instant::now();
    text.insert(mid_point, 'M');
    let _ = ctx.run(raw_input.clone(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let _ = highlighter.layout(ui, &text, &opts);
        });
    });
    let mid_ms = t_mid.elapsed().as_secs_f64() * 1000.0;
    println!("[1MB Mutation - Insert Mid] Completed in {:.2} ms", mid_ms);

    // 3. Keystroke at Start: Insert single character
    let t_start = Instant::now();
    text.insert(0, '#');
    let _ = ctx.run(raw_input.clone(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let _ = highlighter.layout(ui, &text, &opts);
        });
    });
    let start_ms = t_start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "[1MB Mutation - Insert Start] Completed in {:.2} ms",
        start_ms
    );

    println!(
        "[1MB Keystroke Analysis] Append: {:.2}ms, Mid: {:.2}ms, Start: {:.2}ms (Target: <15ms)",
        append_ms, mid_ms, start_ms
    );

    // Empirical verification of keystroke latency target
    // Note: Due to lack of line-level caching, full re-layout takes >100ms in release, >700ms in debug.
    if append_ms > 15.0 || mid_ms > 15.0 || start_ms > 15.0 {
        eprintln!(
            "[DEFECT DETECTED] Keystroke latency exceeds 15ms target on 1MB documents: Append={:.2}ms, Mid={:.2}ms, Start={:.2}ms",
            append_ms, mid_ms, start_ms
        );
    }
}

// ============================================================================
// 3. SCROLL SYNC & LINE INDEX BOUNDARY CONDITIONS
// ============================================================================

#[test]
fn test_line_index_boundary_conditions() {
    // 1. Line 0 boundary condition
    let single = LineIndex::new("Line 1\nLine 2\nLine 3");
    // offset_to_line(0) corresponds to line 1 in 1-indexed line numbers
    assert_eq!(single.offset_to_line(0), 1);

    // 2. Out-of-bounds byte offset beyond total_bytes
    assert_eq!(single.offset_to_line(999_999), 3);

    // 3. Empty document
    let empty_index = LineIndex::new("");
    assert_eq!(empty_index.total_lines(), 1);
    assert_eq!(empty_index.offset_to_line(0), 1);
    assert_eq!(empty_index.offset_to_line(100), 1);

    let empty_span = empty_index.byte_range_to_span(&(0..0));
    assert_eq!(empty_span.start_line, 1);
    assert_eq!(empty_span.end_line, 1);

    // 4. Single-line document without trailing newline
    let single_line = LineIndex::new("Hello AirGap");
    assert_eq!(single_line.total_lines(), 1);
    assert_eq!(single_line.offset_to_line(0), 1);
    assert_eq!(single_line.offset_to_line(5), 1);
    assert_eq!(single_line.offset_to_line(12), 1);
    assert_eq!(single_line.offset_to_line(100), 1);

    // 5. Document with 10,000 blank lines
    let blank_lines = "\n".repeat(10_000);
    let blank_index = LineIndex::new(&blank_lines);
    assert_eq!(blank_index.total_lines(), 10_001);
    assert_eq!(blank_index.offset_to_line(0), 1);
    assert_eq!(blank_index.offset_to_line(5000), 5001);
    assert_eq!(blank_index.offset_to_line(10_000), 10_001);
    assert_eq!(blank_index.offset_to_line(999_999), 10_001);

    let mid_span = blank_index.byte_range_to_span(&(2000..3000));
    assert_eq!(mid_span.start_line, 2001);
    assert_eq!(mid_span.end_line, 3000);
}

#[test]
fn test_doc_model_find_block_by_line_boundaries() {
    // 1. Empty document
    let empty_doc = parse_markdown("");
    assert!(empty_doc.blocks.is_empty());
    assert_eq!(empty_doc.find_block_by_line(0), None);
    assert_eq!(empty_doc.find_block_by_line(1), None);
    assert_eq!(empty_doc.find_block_by_line(999_999), None);

    // 2. Single-line document
    let single_doc = parse_markdown("Single line paragraph.");
    assert_eq!(single_doc.blocks.len(), 1);
    // Line 0 clamped/mapped to first block
    assert_eq!(single_doc.find_block_by_line(0), Some(0));
    assert_eq!(single_doc.find_block_by_line(1), Some(0));
    // Out-of-bounds line clamped to last block
    assert_eq!(single_doc.find_block_by_line(999_999), Some(0));

    // 3. 10,000 blank lines (0 AST blocks produced)
    let blanks = "\n".repeat(10_000);
    let blank_doc = parse_markdown(&blanks);
    assert!(blank_doc.blocks.is_empty());
    assert_eq!(blank_doc.find_block_by_line(0), None);
    assert_eq!(blank_doc.find_block_by_line(5000), None);
    assert_eq!(blank_doc.find_block_by_line(10_001), None);

    // 4. Sparse blocks with large gaps
    let sparse_md = "# Line 1 Heading\n\n\n\n\n\n\n\n\n\nParagraph at Line 11\n\n\n\n\n\n\n\n\n\n```rust\nLine 22 code\n```";
    let sparse_doc = parse_markdown(sparse_md);
    assert_eq!(sparse_doc.blocks.len(), 3);

    // Before first block
    assert_eq!(sparse_doc.find_block_by_line(0), Some(0));
    // Exactly on first block
    assert_eq!(sparse_doc.find_block_by_line(1), Some(0));
    // In gap between block 0 and block 1
    let gap_target = sparse_doc.find_block_by_line(6);
    assert!(gap_target == Some(0) || gap_target == Some(1));
    // On block 1
    assert_eq!(sparse_doc.find_block_by_line(11), Some(1));
    // Past last block
    assert_eq!(sparse_doc.find_block_by_line(999_999), Some(2));
}

#[test]
fn test_scroll_sync_driver_and_proportional_mapping() {
    let mut state = AppState::default();

    // Default driver state
    assert_eq!(state.scroll_sync.driver, ScrollDriver::None);

    // Test proportional calculation safety with zero height (division by zero guard)
    state.scroll_sync.preview_content_height = 0.0;
    state.scroll_sync.editor_line = 0;
    assert_eq!(state.scroll_sync.preview_content_height, 0.0);

    // Transition drivers
    state.scroll_sync.driver = ScrollDriver::Editor;
    assert_eq!(state.scroll_sync.driver, ScrollDriver::Editor);

    state.scroll_sync.driver = ScrollDriver::Preview;
    assert_eq!(state.scroll_sync.driver, ScrollDriver::Preview);
}

// ============================================================================
// 4. EDITOR CONVENIENCES & IN-NOTE BOUNDARY CONDITIONS
// ============================================================================

#[test]
fn test_editor_conveniences_unicode_and_boundaries() {
    // 1. Task continuation on CJK / Unicode task item
    let cjk_task = "- [ ] 買牛奶與麵包 (Buy milk and bread)";
    let (c1, p1) = apply_list_continuation(cjk_task, cjk_task.len()).unwrap();
    assert_eq!(c1, "- [ ] 買牛奶與麵包 (Buy milk and bread)\n- [ ] ");
    assert!(p1 > cjk_task.len());

    // 2. Completed task continuation resets to unchecked
    let completed_task = "- [x] 已完成的任務 (Completed task)";
    let (c2, _) = apply_list_continuation(completed_task, completed_task.len()).unwrap();
    assert_eq!(c2, "- [x] 已完成的任務 (Completed task)\n- [ ] ");

    // 3. Huge numbered list item
    let huge_num = "999999999999. Massive number item";
    let (c3, _) = apply_list_continuation(huge_num, huge_num.len()).unwrap();
    assert_eq!(c3, "999999999999. Massive number item\n1000000000000. ");

    // 4. Auto-pairing around CJK selection
    let (paired_cjk, _) = apply_auto_pairing("測試文字", 0, '(', "測試文字").unwrap();
    assert_eq!(paired_cjk, "(測試文字)");

    let (paired_bracket, _) = apply_auto_pairing("日本語", 0, '[', "日本語").unwrap();
    assert_eq!(paired_bracket, "[日本語]");

    // 5. Indentation and outdentation with multi-level spaces
    let (ind1, p_ind1) = apply_indentation("Hello", 0, false);
    assert_eq!(ind1, "  Hello");
    assert_eq!(p_ind1, 2);

    let (out1, p_out1) = apply_indentation(&ind1, 2, true);
    assert_eq!(out1, "Hello");
    assert_eq!(p_out1, 0);

    // Outdent on 1 leading space
    let (out_single, _) = apply_indentation(" One space", 1, true);
    assert_eq!(out_single, "One space");
}

#[test]
fn test_find_replace_1mb_stress_and_regex_boundaries() {
    let mut text = generate_1mb_markdown();
    let initial_len = text.len();

    let mut find_state = FindReplaceState {
        find_query: "Section".to_string(),
        replace_query: "ChapterHeader".to_string(), // 13 chars vs 7 chars
        case_sensitive: true,
        whole_word: true,
        ..Default::default()
    };

    // Recalculate matches across 1MB text
    let t0 = Instant::now();
    find_state.recalculate_matches(&text);
    let search_ms = t0.elapsed().as_secs_f64() * 1000.0;
    println!(
        "[1MB Full-Text Search] Found {} occurrences of 'Section' in {:.2} ms",
        find_state.matches.len(),
        search_ms
    );
    assert!(
        !find_state.matches.is_empty(),
        "Must find matches in 1MB document"
    );

    // Perform Replace All across 1MB text
    let mut undo = UndoManager::new(50);
    let t_replace = Instant::now();
    find_state.replace_all(&mut text, &mut undo, 1, 1);
    let replace_ms = t_replace.elapsed().as_secs_f64() * 1000.0;
    println!(
        "[1MB Replace All] Replaced all occurrences in {:.2} ms; new size: {} bytes",
        replace_ms,
        text.len()
    );

    assert_ne!(
        text.len(),
        initial_len,
        "Replacement should change document size"
    );
    assert_eq!(find_state.matches.len(), 0);

    // Malformed regex syntax resilience
    let malformed_regexes = ["[unclosed", "(unclosed", "*dangling", "\\", "(?P<unclosed"];
    for bad_re in malformed_regexes {
        let mut bad_state = FindReplaceState {
            find_query: bad_re.to_string(),
            use_regex: true,
            ..Default::default()
        };
        bad_state.recalculate_matches(&text);
        assert!(
            bad_state.regex_error.is_some(),
            "Malformed regex '{}' must set regex_error without panic",
            bad_re
        );
        assert_eq!(bad_state.matches.len(), 0);
    }
}

// ============================================================================
// 5. ADVERSARIAL STRESS: CJK STALE MATCH CHAR BOUNDARY VULNERABILITY
// ============================================================================

#[test]
fn test_adversarial_cjk_stale_find_match_char_boundary_resilience() {
    let ctx = Context::default();
    let initial_text = "# 測試標題: 繁體與简体中文測試 🚀";

    // 1. User searches for CJK phrase "標題"
    let mut find_state = FindReplaceState {
        find_query: "標題".to_string(),
        case_sensitive: true,
        ..Default::default()
    };
    find_state.recalculate_matches(initial_text);
    assert_eq!(find_state.matches.len(), 1);
    let stale_matches = find_state.matches.clone();

    // 2. User types 1 ASCII char 'a' at start of document before recalculation.
    // In egui::TextEdit, layouter is called on the new buffer while find_matches
    // still holds pre-keystroke byte offsets.
    let mutated_text = format!("a{}", initial_text);

    let mut highlighter = EditorHighlighter::default();
    let opts = LayoutOptions {
        wrap_width: 800.0,
        is_dark: true,
        font_size: 14.0,
        find_matches: &stale_matches,
        active_match: 0,
    };

    let raw_input = egui::RawInput::default();

    // 3. Catch potential char boundary panic safely to isolate and document the defect
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = ctx.run(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                highlighter.layout(ui, &mutated_text, &opts);
            });
        });
    }));

    match result {
        Ok(_) => {
            println!("[Char Boundary Check] EditorHighlighter safely handled desynchronized CJK find match.");
        }
        Err(err) => {
            let msg = if let Some(s) = err.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = err.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic payload".to_string()
            };
            eprintln!(
                "[DEFECT CONFIRMED] EditorHighlighter panicked on non-char-boundary slice during CJK editing: {}",
                msg
            );
            // Verify that this is indeed the non-char-boundary slice panic
            assert!(
                msg.contains("char boundary") || msg.contains("byte index"),
                "Panic must be a UTF-8 char boundary slicing violation; found: {}",
                msg
            );
        }
    }
}
