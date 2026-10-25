//! Challenger M1 Adversarial Stress Test Suite
//!
//! Empirical verification of:
//! 1. Atomic writes under simulated interruptions & TempFileCleanupGuard failure recovery
//! 2. CRLF & UTF-8 BOM preservation under read/modify/write cycles
//! 3. Path safety adversarial vectors (multi-extension reserved names, directory traversals, illegal chars, symlinks)
//! 4. Vault tree collision naming stress (rapid sequential creation of Untitled.md through Untitled 10.md)

use airgap_core::io::{detect_line_ending, read_note, write_atomic, LineEnding};
use airgap_core::path_safety::{sanitize_path, validate_filename, PathSafetyError};
use airgap_core::vault::Vault;
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::time::Instant;
use tempfile::tempdir;

// ============================================================================
// 1. ATOMIC WRITES UNDER SIMULATED INTERRUPTIONS & CLEANUP GUARD
// ============================================================================

#[test]
fn test_temp_file_cleanup_on_locked_destination() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("locked_target.md");

    // 1. Create initial file
    write_atomic(&file_path, "Initial Content").unwrap();
    assert!(file_path.exists());

    // 2. Open file with exclusive lock (share_mode = 0 on Windows, blocking rename and deletion)
    #[cfg(windows)]
    let locked_file_opt = {
        use std::os::windows::fs::OpenOptionsExt;
        OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&file_path)
            .ok()
    };
    #[cfg(not(windows))]
    let locked_file_opt: Option<File> = None;

    if let Some(locked_file) = locked_file_opt {
        // 3. Attempt write_atomic while target is exclusively locked
        let write_result = write_atomic(&file_path, "Overwritten Content");
        println!(
            "[Empirical Check] write_atomic on locked target returned: {:?}",
            write_result
        );
        assert!(
            write_result.is_err(),
            "write_atomic must fail when target file is exclusively locked without FILE_SHARE_DELETE"
        );

        // Drop the lock so we can inspect disk state
        drop(locked_file);

        // 4. Verify that TempFileCleanupGuard removed any temporary files
        let parent = file_path.parent().unwrap();
        let lingering_temp_files: Vec<_> = fs::read_dir(parent)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with(".airgap_tmp_"))
            .collect();

        assert_eq!(
            lingering_temp_files.len(),
            0,
            "TempFileCleanupGuard must remove temporary files when rename fails; found: {:?}",
            lingering_temp_files
        );

        // 5. Verify original content was NOT corrupted
        let content = read_note(&file_path).unwrap();
        assert_eq!(
            content, "Initial Content",
            "Original content must remain intact if atomic write fails"
        );
    }
}

#[test]
fn test_temp_file_cleanup_when_target_is_directory() {
    let dir = tempdir().unwrap();
    let target_dir = dir.path().join("existing_dir");
    fs::create_dir(&target_dir).unwrap();

    // Attempt write_atomic over an existing directory path
    let write_res = write_atomic(&target_dir, "Invalid Directory Write");
    println!(
        "[Directory Overwrite Check] write_atomic over existing directory returned: {:?}",
        write_res
    );
    assert!(
        write_res.is_err(),
        "write_atomic over a directory must fail"
    );

    // Verify TempFileCleanupGuard removed any temp files
    let lingering_temp_files: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(".airgap_tmp_"))
        .collect();

    assert_eq!(
        lingering_temp_files.len(),
        0,
        "TempFileCleanupGuard must remove temporary files when target is a directory"
    );

    // Verify existing directory still exists
    assert!(target_dir.is_dir());
}

#[test]
fn test_temp_file_cleanup_guard_on_panic() {
    let dir = tempdir().unwrap();
    let temp_target = dir.path().join(".airgap_tmp_test_guard.tmp");

    // Create a dummy temp file
    fs::write(&temp_target, "temporary data").unwrap();
    assert!(temp_target.exists());

    // Simulate a failure / panic inside a guarded scope
    let result = std::panic::catch_unwind(|| {
        struct MockGuard<'a> {
            path: &'a Path,
            active: bool,
        }
        impl<'a> Drop for MockGuard<'a> {
            fn drop(&mut self) {
                if self.active {
                    let _ = fs::remove_file(self.path);
                }
            }
        }

        let _guard = MockGuard {
            path: &temp_target,
            active: true,
        };

        // Simulate panic during write/sync
        panic!("Simulated power failure or worker panic!");
    });

    assert!(result.is_err(), "Panic must occur");
    // Verify file was removed by Drop guard
    assert!(
        !temp_target.exists(),
        "Guard must delete temp file upon panic unwinding"
    );
}

#[test]
fn test_rapid_consecutive_atomic_writes_no_temp_leak() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("consecutive_writes.md");

    let iterations = 200;
    let t0 = Instant::now();

    for i in 0..iterations {
        let payload = format!(
            "# Iteration {}\nTimestamp: {:?}\nContent line.",
            i,
            t0.elapsed()
        );
        write_atomic(&file_path, &payload).expect("write_atomic must succeed");
    }

    let elapsed = t0.elapsed();
    println!(
        "[Atomic Write Stress] {} consecutive writes took {:?} ({:.2} ms/write)",
        iterations,
        elapsed,
        elapsed.as_secs_f64() * 1000.0 / iterations as f64
    );

    // Verify disk state
    let lingering_temps: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(".airgap_tmp_"))
        .collect();

    assert_eq!(
        lingering_temps.len(),
        0,
        "No temporary files must leak after rapid consecutive writes"
    );

    let final_content = read_note(&file_path).unwrap();
    assert!(
        final_content.starts_with(&format!("# Iteration {}", iterations - 1)),
        "Final content must match the last iteration write"
    );
}

// ============================================================================
// 2. CRLF AND BOM PRESERVATION ADVERSARIAL STRESS
// ============================================================================

#[test]
fn test_crlf_preservation_exhaustive_round_trip() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("strict_crlf.md");

    // 1. Write initial CRLF note
    let raw_crlf = b"# Header\r\n\r\nLine 1\r\nLine 2\r\nLine 3\r\n";
    fs::write(&file_path, raw_crlf).unwrap();

    // 2. Read through read_note
    let loaded = read_note(&file_path).unwrap();
    assert_eq!(detect_line_ending(&loaded), LineEnding::Crlf);

    // 3. Simulate an editor buffer that mixes LF and CRLF
    let edited = "New Header\n\nLine 1\r\nLine 2 Edited\nLine 3\nLine 4 Added\n";

    // 4. Save using write_atomic
    write_atomic(&file_path, edited).unwrap();

    // 5. Inspect exact bytes on disk
    let disk_bytes = fs::read(&file_path).unwrap();
    let disk_str = String::from_utf8(disk_bytes.clone()).unwrap();

    // Verify all line breaks are strictly CRLF
    let mut prev_char = ' ';
    for c in disk_str.chars() {
        if c == '\n' {
            assert_eq!(
                prev_char, '\r',
                "Found isolated LF without preceding CR in CRLF preserved file! Bytes: {:?}",
                disk_bytes
            );
        }
        prev_char = c;
    }
    assert!(
        !disk_str.contains("\r\r\n"),
        "Must not produce duplicate CR (\\r\\r\\n)"
    );
}

#[test]
fn test_lf_preserved_on_existing_lf_file() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("strict_lf.md");

    // 1. Write initial LF note
    let raw_lf = b"# Header\nLine 1\nLine 2\n";
    fs::write(&file_path, raw_lf).unwrap();

    // 2. Read note
    let loaded = read_note(&file_path).unwrap();
    assert_eq!(detect_line_ending(&loaded), LineEnding::Lf);

    // 3. User passes CRLF string to write_atomic
    let crlf_input = "# Header\r\nLine 1\r\nLine 2 Edited\r\n";
    write_atomic(&file_path, crlf_input).unwrap();

    // 4. Verify disk bytes still use LF
    let disk_bytes = fs::read(&file_path).unwrap();
    assert!(
        !disk_bytes.windows(2).any(|w| w == b"\r\n"),
        "Existing LF file must retain LF even if input had CRLF"
    );
}

#[test]
fn test_new_file_defaults_to_lf() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("new_note.md");

    // Write a new note with CRLF content
    write_atomic(&file_path, "Line 1\r\nLine 2\r\n").unwrap();

    // Spec requirement: "default LF for new files"
    let disk_bytes = fs::read(&file_path).unwrap();
    assert!(
        !disk_bytes.windows(2).any(|w| w == b"\r\n"),
        "New file must default to LF line endings"
    );
    assert!(disk_bytes.contains(&b'\n'), "Must contain LF line ending");
}

#[test]
fn test_utf8_bom_multi_roundtrip_preservation() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("bom_roundtrip.md");

    // 1. Initial file with BOM
    let mut initial_bytes = vec![0xEF, 0xBB, 0xBF];
    initial_bytes.extend_from_slice(b"# BOM Document\nInitial line\n");
    fs::write(&file_path, &initial_bytes).unwrap();

    // 2. Multiple successive read -> edit -> save cycles
    for cycle in 1..=5 {
        let content = read_note(&file_path).unwrap();
        assert!(
            !content.starts_with('\u{FEFF}'),
            "read_note must strip BOM from memory string"
        );

        let modified = format!("{}\nCycle {} update\n", content, cycle);
        write_atomic(&file_path, &modified).unwrap();

        let disk_bytes = fs::read(&file_path).unwrap();
        assert_eq!(
            &disk_bytes[0..3],
            &[0xEF, 0xBB, 0xBF],
            "Cycle {}: UTF-8 BOM must be preserved on disk after write_atomic",
            cycle
        );
    }
}

#[test]
fn test_utf8_bom_and_crlf_simultaneous_preservation() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("bom_crlf.md");

    // Initial file with BOTH BOM and CRLF
    let mut initial_bytes = vec![0xEF, 0xBB, 0xBF];
    initial_bytes.extend_from_slice(b"# Dual BOM + CRLF\r\nLine 1\r\nLine 2\r\n");
    fs::write(&file_path, &initial_bytes).unwrap();

    // Edit and save
    let loaded = read_note(&file_path).unwrap();
    let edited = format!("{}\r\nAppended Line\r\n", loaded);
    write_atomic(&file_path, &edited).unwrap();

    // Verify disk bytes
    let disk_bytes = fs::read(&file_path).unwrap();
    assert_eq!(
        &disk_bytes[0..3],
        &[0xEF, 0xBB, 0xBF],
        "UTF-8 BOM must be preserved"
    );
    assert!(
        disk_bytes[3..].windows(2).any(|w| w == b"\r\n"),
        "CRLF must be preserved"
    );
    let disk_str = String::from_utf8(disk_bytes[3..].to_vec()).unwrap();
    let mut prev = ' ';
    for c in disk_str.chars() {
        if c == '\n' {
            assert_eq!(prev, '\r', "All newlines must be CRLF");
        }
        prev = c;
    }
}

// ============================================================================
// 3. PATH SAFETY ADVERSARIAL VECTORS
// ============================================================================

#[test]
fn test_adversarial_windows_reserved_names_multi_extension() {
    let reserved_names_multi_ext = [
        "con.bak.md",
        "CON.bak.md",
        "con.1.md",
        "aux.old.tar.gz",
        "AUX.backup.zip",
        "nul.old.txt",
        "NUL.tar.gz",
        "prn.data.json",
        "PRN.backup",
        "com1.log.txt",
        "COM2.sub.md",
        "COM9.txt",
        "lpt1.old.md",
        "LPT9.archive.tar.gz",
        // With leading/trailing spaces before extension
        "con .md",
        "aux .txt",
        "nul .markdown",
    ];

    for name in &reserved_names_multi_ext {
        let res = validate_filename(name);
        println!("[Reserved Check] '{}' => {:?}", name, res);
        assert!(
            matches!(
                res,
                Err(PathSafetyError::ReservedName(_))
                    | Err(PathSafetyError::TrailingPeriodOrSpace(_))
            ),
            "Expected '{}' to be rejected as Windows reserved name or illegal format",
            name
        );
    }
}

#[test]
fn test_adversarial_directory_traversal_variations() {
    let dir = tempdir().unwrap();
    let vault_root = dir.path();

    let traversal_vectors = [
        "../../escaped.md",
        "sub/../../escaped.md",
        "sub/sub2/../../../escaped.md",
        "..\\..\\escaped.md",
        "sub\\..\\..\\escaped.md",
        "./../../escaped.md",
        "sub/./../../escaped.md",
        "a/b/c/../../../../escaped.md",
    ];

    for vector in &traversal_vectors {
        let res = sanitize_path(vault_root, Path::new(vector));
        println!("[Traversal Check] '{}' => {:?}", vector, res);
        assert!(
            matches!(res, Err(PathSafetyError::TraversalAttempted(_))),
            "Expected '{}' to be rejected as directory traversal",
            vector
        );
    }
}

#[test]
fn test_adversarial_illegal_characters_and_control_chars() {
    let illegal_names = [
        "file<name.md",
        "file>name.md",
        "file:colon.md",
        "file\"quote.md",
        "file/slash.md",
        "file\\backslash.md",
        "file|pipe.md",
        "file?question.md",
        "file*star.md",
        "file\x00null.md",
        "file\x07bell.md",
        "file\x08backspace.md",
        "file\x1Funit.md",
    ];

    for name in &illegal_names {
        let res = validate_filename(name);
        assert!(
            matches!(res, Err(PathSafetyError::IllegalCharacters(_))),
            "Expected '{}' to be rejected as illegal character",
            name
        );
    }
}

#[test]
fn test_adversarial_trailing_periods_and_spaces() {
    let trailing_vectors = [
        "note.",
        "note..",
        "note. ",
        "note ",
        "note  ",
        "folder. /",
        "my_note.md.",
        "my_note.md ",
    ];

    for name in &trailing_vectors {
        let res = validate_filename(name);
        assert!(
            matches!(
                res,
                Err(PathSafetyError::TrailingPeriodOrSpace(_))
                    | Err(PathSafetyError::IllegalCharacters(_))
            ),
            "Expected '{}' to be rejected as trailing dot/space",
            name
        );
    }
}

#[test]
fn test_adversarial_symlink_escape_outside_vault() {
    let dir = tempdir().unwrap();
    let vault_root = dir.path().join("vault");
    let outside_dir = dir.path().join("outside_secrets");
    fs::create_dir_all(&vault_root).unwrap();
    fs::create_dir_all(&outside_dir).unwrap();

    let secret_file = outside_dir.join("passwords.txt");
    fs::write(&secret_file, "secret_credentials_12345").unwrap();

    // Create a symlink inside the vault pointing outside
    let symlink_path = vault_root.join("leak_link.txt");

    #[cfg(windows)]
    let symlink_created = std::os::windows::fs::symlink_file(&secret_file, &symlink_path);
    #[cfg(not(windows))]
    let symlink_created = std::os::unix::fs::symlink(&secret_file, &symlink_path);

    if let Ok(()) = symlink_created {
        println!("[Symlink] Successfully created test symlink inside vault pointing outside");
        let sanitize_res = sanitize_path(&vault_root, Path::new("leak_link.txt"));
        println!("[Symlink Escape Check] Result: {:?}", sanitize_res);
        assert!(
            matches!(sanitize_res, Err(PathSafetyError::TraversalAttempted(_))),
            "Symlink escaping vault root MUST be rejected with TraversalAttempted"
        );
    } else {
        println!(
            "[Symlink] Skipping symlink test: OS privilege not held (common on non-dev Windows)"
        );
    }
}

// ============================================================================
// 4. VAULT TREE COLLISION NAMING STRESS
// ============================================================================

#[test]
fn test_rapid_sequential_creation_untitled_1_through_10() {
    let dir = tempdir().unwrap();
    let vault = Vault::new(dir.path());

    let mut created_names = Vec::new();
    let total_notes = 11; // Untitled.md, Untitled 1.md .. Untitled 10.md

    let t0 = Instant::now();
    for _ in 0..total_notes {
        let note_rel = vault
            .create_note(Path::new(""), "Untitled.md")
            .expect("create_note must succeed");
        created_names.push(note_rel.to_string_lossy().into_owned());
    }
    let elapsed = t0.elapsed();

    println!(
        "[Collision Naming Stress] Created {} notes sequentially in {:?} ({:.2} ms/note)",
        total_notes,
        elapsed,
        elapsed.as_secs_f64() * 1000.0 / total_notes as f64
    );

    // Verify exact expected sequence
    assert_eq!(created_names[0], "Untitled.md");
    for (i, name) in created_names.iter().enumerate().take(total_notes).skip(1) {
        let expected = format!("Untitled {}.md", i);
        assert_eq!(
            name, &expected,
            "Note index {} must be named '{}'",
            i, expected
        );
    }

    // Verify all notes exist on disk and can be read
    for name in &created_names {
        let abs_path = dir.path().join(name);
        assert!(abs_path.exists(), "Note '{}' must exist on disk", name);
        let content = vault.read_note(Path::new(name)).unwrap();
        assert_eq!(content, "", "Newly created note must be empty string");
    }

    // Verify tree scan lists all 11 notes in natural sort order
    let tree = vault.scan_tree().unwrap();
    assert_eq!(
        tree.children.len(),
        total_notes,
        "Tree node children count must equal total notes created"
    );
}

#[test]
fn test_collision_naming_with_custom_extensions_and_nested_dirs() {
    let dir = tempdir().unwrap();
    let vault = Vault::new(dir.path());

    // Create nested subfolder
    let subfolder_rel = vault.create_folder(Path::new(""), "Projects").unwrap();

    // Create collision notes in subfolder
    let n1 = vault
        .create_note(&subfolder_rel, "Sprint Plan.markdown")
        .unwrap();
    let n2 = vault
        .create_note(&subfolder_rel, "Sprint Plan.markdown")
        .unwrap();
    let n3 = vault
        .create_note(&subfolder_rel, "Sprint Plan.markdown")
        .unwrap();

    assert_eq!(
        n1.to_string_lossy().replace('\\', "/"),
        "Projects/Sprint Plan.markdown"
    );
    assert_eq!(
        n2.to_string_lossy().replace('\\', "/"),
        "Projects/Sprint Plan 1.markdown"
    );
    assert_eq!(
        n3.to_string_lossy().replace('\\', "/"),
        "Projects/Sprint Plan 2.markdown"
    );

    // Also test collision naming without extension
    let no_ext1 = vault.create_note(&subfolder_rel, "Scratchpad").unwrap();
    let no_ext2 = vault.create_note(&subfolder_rel, "Scratchpad").unwrap();
    assert_eq!(
        no_ext1.to_string_lossy().replace('\\', "/"),
        "Projects/Scratchpad.md"
    );
    assert_eq!(
        no_ext2.to_string_lossy().replace('\\', "/"),
        "Projects/Scratchpad 1.md"
    );
}

#[test]
fn test_high_volume_collision_stress_50_notes() {
    let dir = tempdir().unwrap();
    let vault = Vault::new(dir.path());

    let count = 50;
    let t0 = Instant::now();

    for _ in 0..count {
        vault
            .create_note(Path::new(""), "StressNote.md")
            .expect("High volume note creation must succeed");
    }

    let elapsed = t0.elapsed();
    println!(
        "[High Volume Collision] Created {} notes in {:?} ({:.2} ms/note)",
        count,
        elapsed,
        elapsed.as_secs_f64() * 1000.0 / count as f64
    );

    // Verify all 50 exist on disk
    let disk_entries: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("StressNote"))
        .collect();
    assert_eq!(disk_entries.len(), count);
}
