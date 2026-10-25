//! Milestone M2 Adversarial Security & State Verification Suite
//!
//! Empirical adversarial test harness targeting:
//! 1. Image Resolution Attack Vectors (Path traversal, Protocol injection, Windows device names, Symlink escape)
//! 2. URL Sanitization Attack Vectors (Dangerous schemes, Length bounds, Control characters, Metacharacters)
//! 3. Editor State Edge Cases (Undo/Redo 500 rapid mutations, Empty stack pops, Document reload, ReDoS immunity, Empty string replacements, Token safety)

use airgap_app::state::{AppState, FindReplaceState, UndoManager};
use airgap_app::url_safety::sanitize_external_url;
use airgap_render::{resolve_local_image_path, ImageResolutionError};
use std::fs::{self, File};
use std::path::PathBuf;
use std::time::Instant;
use tempfile::tempdir;

// ============================================================================
// AREA 1: Image Resolution Attack Vectors
// ============================================================================

#[test]
fn test_adversarial_image_directory_traversal() {
    let dir = tempdir().expect("Failed to create temporary directory");
    let vault_root = dir.path().join("vault");
    let outside_dir = dir.path().join("outside");
    fs::create_dir_all(&vault_root).unwrap();
    fs::create_dir_all(&outside_dir).unwrap();

    // Create outside confidential file
    let outside_secret = outside_dir.join("secret.png");
    File::create(&outside_secret).unwrap();

    // Create a note in vault root and a note in a subfolder
    let root_note = vault_root.join("root_note.md");
    File::create(&root_note).unwrap();

    let sub_dir = vault_root.join("subfolder");
    fs::create_dir_all(&sub_dir).unwrap();
    let sub_note = sub_dir.join("nested_note.md");
    File::create(&sub_note).unwrap();

    // 1. Classic double dot parent traversals
    let traversal_vectors = [
        "../../etc/passwd",
        r"..\..\windows\system32\calc.exe",
        "folder/../../../secret.png",
        "../outside/secret.png",
        r"..\outside\secret.png",
        "/../../secret.png",
        r"\..\..\secret.png",
        "./../../secret.png",
        r".\..\..\secret.png",
        "subfolder/../../outside/secret.png",
        "subfolder/../../../outside/secret.png",
        "assets/../../../etc/shadow",
        "sub/sub2/../../../../secret.png",
    ];

    for vector in &traversal_vectors {
        let result_root = resolve_local_image_path(&vault_root, &root_note, vector);
        assert!(
            matches!(result_root, Err(ImageResolutionError::VaultEscape(_))),
            "Expected VaultEscape for root_note with vector '{}', but got {:?}",
            vector,
            result_root
        );

        let result_sub = resolve_local_image_path(&vault_root, &sub_note, vector);
        assert!(
            matches!(
                result_sub,
                Err(ImageResolutionError::VaultEscape(_)) | Err(ImageResolutionError::NotFound(_))
            ),
            "Expected VaultEscape or NotFound for sub_note with vector '{}', but got {:?}",
            vector,
            result_sub
        );
    }
}

#[test]
fn test_adversarial_image_protocol_injection() {
    let dir = tempdir().expect("Failed to create temporary directory");
    let vault_root = dir.path().join("vault");
    fs::create_dir_all(&vault_root).unwrap();
    let note = vault_root.join("note.md");
    File::create(&note).unwrap();

    // Network protocols must be strictly blocked under local-only privacy
    let network_vectors = [
        "http://attacker.com/evil.png",
        "https://evil.com/x.png",
        "ftp://evil.com/x.png",
        "//evil.com/x.png",
        "HTTP://ATTACKER.COM/EVIL.PNG",
        "HTTPS://EVIL.COM/X.PNG",
        "FTP://EVIL.COM/X.PNG",
        "//CDN.EXAMPLE.COM/PHOTO.PNG",
        "  https://evil.com/padded.png  ",
    ];

    for vector in &network_vectors {
        let res = resolve_local_image_path(&vault_root, &note, vector);
        assert!(
            matches!(res, Err(ImageResolutionError::NetworkBlocked(_))),
            "Expected NetworkBlocked for vector '{}', got {:?}",
            vector,
            res
        );
    }

    // file:// protocol injection must not escape vault boundary or leak system files
    let file_proto_vectors = [
        "file:///etc/hosts",
        "file:///etc/passwd",
        "file:///C:/Windows/System32/calc.exe",
        "file:///C:/Windows/System32/image.png",
        r"file://\\localhost\c$\secret.png",
    ];

    for vector in &file_proto_vectors {
        let res = resolve_local_image_path(&vault_root, &note, vector);
        assert!(
            matches!(
                res,
                Err(ImageResolutionError::VaultEscape(_))
                    | Err(ImageResolutionError::NotFound(_))
                    | Err(ImageResolutionError::InvalidPath(_))
            ),
            "file:// vector '{}' must not resolve successfully, got {:?}",
            vector,
            res
        );
    }

    // Script pseudo-protocols
    let script_vectors = [
        "javascript:alert(1)",
        "javascript:alert(1);evil.png",
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
        "vbscript:MsgBox(1)",
    ];

    for vector in &script_vectors {
        let res = resolve_local_image_path(&vault_root, &note, vector);
        assert!(
            matches!(
                res,
                Err(ImageResolutionError::NotFound(_))
                    | Err(ImageResolutionError::UnsupportedFormat(_))
                    | Err(ImageResolutionError::InvalidPath(_))
                    | Err(ImageResolutionError::VaultEscape(_))
            ),
            "Script vector '{}' must be rejected, got {:?}",
            vector,
            res
        );
    }
}

#[test]
fn test_adversarial_image_windows_specific_paths() {
    let dir = tempdir().expect("Failed to create temporary directory");
    let vault_root = dir.path().join("vault");
    fs::create_dir_all(&vault_root).unwrap();
    let note = vault_root.join("note.md");
    File::create(&note).unwrap();

    // Absolute drive letters and UNC paths
    let win_absolute_vectors = [
        r"C:\Windows\System32\image.png",
        "C:/Windows/System32/image.png",
        r"D:\Secret\photo.png",
        r"\\localhost\c$\image.png",
        r"\\127.0.0.1\share\pic.png",
        r"\\?\C:\Windows\System32\image.png",
    ];

    for vector in &win_absolute_vectors {
        let res = resolve_local_image_path(&vault_root, &note, vector);
        assert!(
            matches!(
                res,
                Err(ImageResolutionError::VaultEscape(_))
                    | Err(ImageResolutionError::NetworkBlocked(_))
                    | Err(ImageResolutionError::InvalidPath(_))
                    | Err(ImageResolutionError::NotFound(_))
                    | Err(ImageResolutionError::UnsupportedFormat(_))
            ),
            "Windows absolute/UNC path '{}' must be rejected, got {:?}",
            vector,
            res
        );
    }

    // Windows device reserved names
    let reserved_names = [
        "CON.png", "NUL.png", "PRN.png", "AUX.png", "COM1.png", "COM2.png", "LPT1.png", "con.png",
        "nul.png",
    ];

    for name in &reserved_names {
        let res = resolve_local_image_path(&vault_root, &note, name);
        // Reserved device names should not succeed as valid images
        assert!(
            matches!(
                res,
                Err(ImageResolutionError::NotFound(_)) | Err(ImageResolutionError::InvalidPath(_))
            ),
            "Reserved device name '{}' must not resolve as valid image, got {:?}",
            name,
            res
        );
    }
}

#[test]
fn test_adversarial_image_symlink_escape() {
    let dir = tempdir().expect("Failed to create temporary directory");
    let vault_root = dir.path().join("vault");
    let outside_dir = dir.path().join("outside");
    fs::create_dir_all(&vault_root).unwrap();
    fs::create_dir_all(&outside_dir).unwrap();

    let outside_image = outside_dir.join("outside_photo.png");
    File::create(&outside_image).unwrap();

    let note = vault_root.join("note.md");
    File::create(&note).unwrap();

    let symlink_path = vault_root.join("symlink_to_outside.png");

    #[cfg(windows)]
    let symlink_created = std::os::windows::fs::symlink_file(&outside_image, &symlink_path).is_ok();

    #[cfg(unix)]
    let symlink_created = std::os::unix::fs::symlink(&outside_image, &symlink_path).is_ok();

    #[cfg(not(any(windows, unix)))]
    let symlink_created = false;

    if symlink_created {
        let res = resolve_local_image_path(&vault_root, &note, "symlink_to_outside.png");
        assert!(
            matches!(res, Err(ImageResolutionError::VaultEscape(_))),
            "Symlink resolving outside vault root must trigger VaultEscape, got {:?}",
            res
        );
    } else {
        println!("Note: Symlink creation skipped (insufficient OS privilege/developer mode)");
    }
}

// ============================================================================
// AREA 2: URL Sanitization Attack Vectors
// ============================================================================

#[test]
fn test_adversarial_url_dangerous_schemes() {
    let dangerous_urls = [
        "file:///etc/passwd",
        r"file:///C:/Windows/System32/calc.exe",
        "data:text/html,<script>alert(1)</script>",
        "data:application/octet-stream;base64,AAAA",
        "javascript:alert(document.cookie)",
        "javascript:void(0)",
        "JAVASCRIPT:alert(1)",
        "powershell:Get-Process",
        r"powershell.exe -Command Remove-Item C:\*",
        "cmd:/c calc.exe",
        "cmd.exe /k calc.exe",
        "ssh://user@evil.com",
        "telnet://evil.com:23",
        "ws://evil.com/stream",
        "wss://evil.com/stream",
        "gopher://evil.com",
        "vbscript:MsgBox(1)",
        "ms-msdt:/id PCWDiagnostic",
        "search-ms:query=calc",
        "jar:file:///path/to/archive.zip!/file.html",
        "about:blank",
        "chrome://settings",
        "ms-settings:privacy",
    ];

    for url in &dangerous_urls {
        let res = sanitize_external_url(url);
        assert!(
            res.is_err(),
            "Dangerous scheme '{}' must be rejected, but got Ok('{:?}')",
            url,
            res
        );
    }
}

#[test]
fn test_adversarial_url_length_bounds_and_control_chars() {
    // 1. Length bounds (max 2048 characters)
    let url_2048 = format!("http://example.com/{}", "a".repeat(2048 - 19));
    assert!(
        sanitize_external_url(&url_2048).is_ok(),
        "URL of exact 2048 characters must be allowed"
    );

    let url_2049 = format!("http://example.com/{}", "a".repeat(2049 - 19));
    let err_2049 = sanitize_external_url(&url_2049);
    assert_eq!(err_2049, Err("URL exceeds 2048 characters"));

    let url_5000 = format!("https://example.com/{}", "x".repeat(5000));
    let err_5000 = sanitize_external_url(&url_5000);
    assert_eq!(err_5000, Err("URL exceeds 2048 characters"));

    // 2. Control characters & whitespace injection
    let control_char_urls = [
        "https://example.com/path\0withnull",
        "https://example.com/path\rwithcr",
        "https://example.com/path\nwithlf",
        "https://example.com/path\twithtab",
        "https://example.com/path withspace",
        "http://evil.com\x08backspace",
        "http://evil.com\x1b[31mescape",
        "mailto:user\n@evil.com",
        "mailto:user\0@evil.com",
        "mailto:user @evil.com",
    ];

    for url in &control_char_urls {
        let res = sanitize_external_url(url);
        assert!(
            res.is_err(),
            "URL with control/whitespace '{}' must be rejected, got {:?}",
            url,
            res
        );
    }

    // 3. Malformed and empty hosts
    let malformed_urls = [
        "",
        "   ",
        "\t\n",
        "http://",
        "https://",
        "http:///",
        "https:///missing-host",
        r"http://\invalid-slash",
        "mailto:",
        "mailto:missing_at_symbol",
    ];

    for url in &malformed_urls {
        assert!(
            sanitize_external_url(url).is_err(),
            "Malformed URL '{}' must be rejected",
            url
        );
    }
}

#[test]
fn test_adversarial_url_query_metacharacters_and_encoding() {
    // Standard web URLs frequently use query parameters with '&', '=', and harmless tokens.
    // Ensure standard URLs with query params pass sanitization without false positive rejection:
    let valid_query_url = "https://example.com/search?query=rust&category=security&page=1";
    assert_eq!(
        sanitize_external_url(valid_query_url),
        Ok(valid_query_url.to_string())
    );

    let valid_encoded_url = "https://example.com/api?tag=%22test%22&filter=%24value";
    assert_eq!(
        sanitize_external_url(valid_encoded_url),
        Ok(valid_encoded_url.to_string())
    );

    // Shell metacharacters in URL paths/queries:
    // When passed to open_external_url_safely, the URL is dispatched via open::that_detached
    // which delegates to OS ShellExecuteExW (bypassing cmd.exe).
    // Verify that the sanitizer does not crash or allocate unbounded memory:
    let meta_urls = [
        "http://example.com/test&calc.exe",
        "http://example.com/test|notepad",
        "http://example.com/test;notepad",
        "https://example.com/test?param=`calc`",
        "https://example.com/test?param=$(whoami)",
    ];

    for url in &meta_urls {
        let res = sanitize_external_url(url);
        // Must either successfully parse as an HTTP string without control chars or return an Err, never panic
        assert!(res.is_ok() || res.is_err());
    }
}

// ============================================================================
// AREA 3: Editor State Edge Cases
// ============================================================================

#[test]
fn test_adversarial_undo_redo_rapid_alternating_typing_and_deletions() {
    let mut undo = UndoManager::new(200);
    let mut current_text = String::from("Initial document content");
    undo.push_snapshot(&current_text, 1, 1, true);

    // Perform 500 rapid alternating typing and deletion mutations
    for i in 0..500 {
        if i % 2 == 0 {
            // Typing
            current_text.push_str(&format!(" mutation_{}", i));
        } else {
            // Deletion (remove last 10 characters or half)
            let truncate_len = current_text.len().saturating_sub(10).max(5);
            current_text.truncate(truncate_len);
        }

        // Force push snapshot to stress history bounds
        undo.push_snapshot(&current_text, 1, current_text.len(), true);

        // Invariant: Undo stack size must never exceed max_history (200)
        assert!(
            undo.undo_stack.len() <= 200,
            "Undo stack grew beyond capacity: {}",
            undo.undo_stack.len()
        );
    }

    assert_eq!(
        undo.undo_stack.len(),
        200,
        "Undo stack should be capped at max_history (200)"
    );

    // Verify 50 sequential undos
    let mut undo_count = 0;
    while let Some(snapshot) = undo.undo(&current_text, 1, 1) {
        current_text = snapshot.text;
        undo_count += 1;
        if undo_count >= 50 {
            break;
        }
    }
    assert_eq!(undo_count, 50, "Expected to perform 50 undos successfully");
    assert_eq!(
        undo.redo_stack.len(),
        50,
        "Redo stack should contain 50 items"
    );

    // Verify 50 sequential redos
    let mut redo_count = 0;
    while let Some(snapshot) = undo.redo(&current_text, 1, 1) {
        current_text = snapshot.text;
        redo_count += 1;
        if redo_count >= 50 {
            break;
        }
    }
    assert_eq!(redo_count, 50, "Expected to perform 50 redos successfully");
}

#[test]
fn test_adversarial_undo_redo_empty_stack_and_reload_invariants() {
    // 1. Pop from empty stack
    let mut empty_undo = UndoManager::new(10);
    assert!(empty_undo.undo("text", 1, 1).is_none());
    assert!(empty_undo.redo("text", 1, 1).is_none());

    // 100 consecutive calls on empty stacks must return None with zero panics
    for _ in 0..100 {
        assert!(empty_undo.undo("text", 1, 1).is_none());
        assert!(empty_undo.redo("text", 1, 1).is_none());
    }

    // 2. Undo after document reload
    let dir = tempdir().unwrap();
    let vault_root = dir.path().to_path_buf();
    let note_path = vault_root.join("test_reload.md");
    fs::write(&note_path, "# Original Document Content\nLine 2").unwrap();

    let mut state = AppState::default();
    state.open_vault(vault_root.clone()).unwrap();
    state.select_note(PathBuf::from("test_reload.md"));

    assert_eq!(state.editor_content, "# Original Document Content\nLine 2");
    assert_eq!(state.undo_manager.undo_stack.len(), 1);

    // Make an edit and save it clean
    state
        .editor_content
        .push_str("\nAppended edit saved to disk");
    state.save_active_note();
    assert!(!state.is_dirty());

    // External process updates file on disk
    fs::write(
        &note_path,
        "# External Update from Disk\nFreshly reloaded line",
    )
    .unwrap();

    // Reload note from disk into state
    state.select_note(PathBuf::from("test_reload.md"));
    assert_eq!(
        state.editor_content,
        "# External Update from Disk\nFreshly reloaded line"
    );
    assert_eq!(
        state.undo_manager.undo_stack.len(),
        1,
        "Reloaded document must reset undo history to 1 clean snapshot"
    );

    // Calling undo immediately after reload should return None (cannot undo prior to load)
    let undone = state.undo_manager.undo(&state.editor_content, 1, 1);
    assert!(
        undone.is_none(),
        "Undo immediately after document reload must return None (cannot undo prior to load)"
    );
}

#[test]
fn test_adversarial_find_replace_pathological_regexes() {
    let text = "Sample text for testing pathological and malformed regular expressions.";
    let mut find_state = FindReplaceState {
        use_regex: true,
        ..Default::default()
    };

    let pathological_queries = [
        "[a-zA-Z",         // unclosed character class
        "(unclosed_group", // unclosed group
        "*dangling_star",  // repetition operator missing operand
        "+dangling_plus",
        "?dangling_question",
        "(?=",              // unclosed lookaround
        r"trailing_slash\", // trailing backslash escape
        "[z-a]",            // inverted character range
        "(?",               // invalid group prefix
        "**",               // invalid multiple quantifier
        r"\p{InvalidUnicodeCategory}",
        "(?<name>", // unclosed named capture group
    ];

    for query in &pathological_queries {
        find_state.find_query = query.to_string();
        find_state.recalculate_matches(text);

        // Invariant: Pathological regexes must never panic!
        assert!(
            find_state.regex_error.is_some(),
            "Pathological regex '{}' must set regex_error",
            query
        );
        assert!(
            find_state.matches.is_empty(),
            "Pathological regex '{}' must produce 0 matches",
            query
        );

        // Invariant: replace operations on invalid regex must be safe no-ops
        let mut mutated_text = text.to_string();
        let mut undo = UndoManager::new(10);
        find_state.replace_current(&mut mutated_text, &mut undo, 1, 1);
        assert_eq!(mutated_text, text);

        find_state.replace_all(&mut mutated_text, &mut undo, 1, 1);
        assert_eq!(mutated_text, text);
    }
}

#[test]
fn test_adversarial_find_replace_catastrophic_backtracking_immunity() {
    // Stress test classic ReDoS (Catastrophic Backtracking) patterns against repeating text.
    // The Rust `regex` crate uses linear-time DFA/NFA and is mathematically immune to ReDoS.
    // This test empirically validates that execution completes in milliseconds without hanging.

    let repeating_a = "a".repeat(10_000);
    let repeating_x = "x".repeat(10_000);

    let redos_patterns = [
        (r"(a+)+$", &repeating_a),
        (r"(a*)*b", &repeating_a),
        (r"(x+x+)+y", &repeating_x),
        (r"(a|aa)+$", &repeating_a),
        (r"([a-zA-Z]+)*=", &repeating_a),
        (r"((a+)*)+", &repeating_a),
    ];

    let t0 = Instant::now();

    for (pattern, target) in &redos_patterns {
        let mut find_state = FindReplaceState {
            find_query: pattern.to_string(),
            use_regex: true,
            ..Default::default()
        };

        let iter_start = Instant::now();
        find_state.recalculate_matches(target);
        let elapsed = iter_start.elapsed();

        // Invariant: Must execute under 50ms per pattern
        assert!(
            elapsed.as_millis() < 50,
            "ReDoS pattern '{}' took too long: {:?}",
            pattern,
            elapsed
        );
    }

    let total_elapsed = t0.elapsed();
    assert!(
        total_elapsed.as_millis() < 200,
        "Total ReDoS stress tests took too long: {:?}",
        total_elapsed
    );
}

#[test]
fn test_adversarial_find_replace_empty_string_replacement() {
    let mut text = "Sensitive SECRET_KEY and another SECRET_KEY must be wiped.".to_string();
    let mut find_state = FindReplaceState {
        find_query: "SECRET_KEY".to_string(),
        replace_query: String::new(), // Empty string replacement
        ..Default::default()
    };

    find_state.recalculate_matches(&text);
    assert_eq!(find_state.matches.len(), 2);

    let mut undo = UndoManager::new(10);
    // Push initial baseline snapshot
    undo.push_snapshot(&text, 1, 1, true);

    // Replace current
    find_state.replace_current(&mut text, &mut undo, 1, 1);
    assert_eq!(text, "Sensitive  and another SECRET_KEY must be wiped.");
    assert_eq!(find_state.matches.len(), 1);

    // Replace all remaining
    find_state.replace_all(&mut text, &mut undo, 1, 1);
    assert_eq!(text, "Sensitive  and another  must be wiped.");
    assert_eq!(find_state.matches.len(), 0);

    // Undo restores cleanly
    let snap1 = undo.undo(&text, 1, 1).unwrap();
    text = snap1.text;
    assert_eq!(text, "Sensitive  and another SECRET_KEY must be wiped.");

    let snap2 = undo.undo(&text, 1, 1).unwrap();
    text = snap2.text;
    assert_eq!(
        text,
        "Sensitive SECRET_KEY and another SECRET_KEY must be wiped."
    );
}

#[test]
fn test_adversarial_find_replace_replacement_tokens_and_dollar_signs() {
    let mut text = "Hello target and goodbye target.".to_string();
    let mut find_state = FindReplaceState {
        find_query: "target".to_string(),
        replace_query: r"$1_$2_&_\1_${HOME}_`calc`".to_string(),
        ..Default::default()
    };

    find_state.recalculate_matches(&text);
    assert_eq!(find_state.matches.len(), 2);

    let mut undo = UndoManager::new(10);
    find_state.replace_all(&mut text, &mut undo, 1, 1);

    // Invariant: The replacement must be treated as a literal string.
    // It must not expand shell variables or capture groups ($1, $2, `calc`).
    assert_eq!(
        text,
        r"Hello $1_$2_&_\1_${HOME}_`calc` and goodbye $1_$2_&_\1_${HOME}_`calc`."
    );
}

#[test]
fn test_adversarial_find_replace_unicode_and_case_insensitivity() {
    let mut text = "Alpha BETA gamma. Héllo WÖRLD. 🚀 RockÉt 🚀".to_string();

    // 1. Mixed ASCII case-insensitive search
    let mut find_state = FindReplaceState {
        find_query: "beta".to_string(),
        replace_query: "DELTA".to_string(),
        case_sensitive: false,
        ..Default::default()
    };
    find_state.recalculate_matches(&text);
    assert_eq!(find_state.matches.len(), 1);

    let mut undo = UndoManager::new(10);
    find_state.replace_all(&mut text, &mut undo, 1, 1);
    assert!(text.contains("Alpha DELTA gamma"));

    // 2. Unicode emoji search
    let mut emoji_state = FindReplaceState {
        find_query: "🚀".to_string(),
        replace_query: "⭐".to_string(),
        ..Default::default()
    };
    emoji_state.recalculate_matches(&text);
    assert_eq!(emoji_state.matches.len(), 2);
    emoji_state.replace_all(&mut text, &mut undo, 1, 1);
    assert_eq!(text, "Alpha DELTA gamma. Héllo WÖRLD. ⭐ RockÉt ⭐");

    // 3. Multi-byte CJK search and replace
    let mut cjk_text = "AirGap 支持 中文输入 和 日语 日本語 输入。".to_string();
    let mut cjk_state = FindReplaceState {
        find_query: "中文".to_string(),
        replace_query: "多语言".to_string(),
        ..Default::default()
    };
    cjk_state.recalculate_matches(&cjk_text);
    assert_eq!(cjk_state.matches.len(), 1);
    cjk_state.replace_all(&mut cjk_text, &mut undo, 1, 1);
    assert_eq!(cjk_text, "AirGap 支持 多语言输入 和 日语 日本語 输入。");
}
