//! Tier 3: Cross-Feature Integration Tests
//!
//! Verifies cross-feature interactions: external disk change during active edits,
//! delete to trash with fallback, import conflicts with "keep both" suffix policy,
//! and search queries during active file mutations.

use airgap_core::{
    delete_to_trash, global_search, quick_open_search, read_note, write_atomic, Vault,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn test_external_disk_change_conflict_detection_and_resolution() {
    let dir = tempdir().expect("tempdir");
    let note_path = dir.path().join("conflict_note.md");

    // 1. Initial saved state on disk
    let initial_disk_content = "# Project Brief\nInitial outline.\n";
    write_atomic(&note_path, initial_disk_content).unwrap();

    // 2. User opens note in editor buffer
    let mut in_memory_buffer = initial_disk_content.to_string();
    let initial_load_mtime = fs::metadata(&note_path).unwrap().modified().unwrap();

    // 3. User makes in-memory edits (dirty buffer, unsaved)
    in_memory_buffer.push_str("User added section: Architecture Decisions.\n");
    let is_dirty = in_memory_buffer != initial_disk_content;
    assert!(is_dirty);

    // Sleep briefly to ensure filesystem timestamp increments
    thread::sleep(Duration::from_millis(50));

    // 4. External process (Git pull / external editor) modifies the file on disk
    let external_content = "# Project Brief\nUpdated remotely by coworker.\n";
    fs::write(&note_path, external_content).unwrap();

    // 5. Watcher / Save check detects external change
    let current_disk_content = read_note(&note_path).unwrap();
    let current_disk_mtime = fs::metadata(&note_path).unwrap().modified().unwrap();

    let has_conflict = is_dirty
        && (current_disk_mtime > initial_load_mtime)
        && (current_disk_content != initial_disk_content);

    assert!(
        has_conflict,
        "Failed to detect concurrent external disk modification conflict!"
    );

    // Test Resolution 1: "Keep Mine" (User forces save of in-memory buffer)
    {
        let keep_mine_dir = tempdir().unwrap();
        let test_file = keep_mine_dir.path().join("test.md");
        fs::write(&test_file, external_content).unwrap();

        write_atomic(&test_file, &in_memory_buffer).unwrap();
        assert_eq!(read_note(&test_file).unwrap(), in_memory_buffer);
    }

    // Test Resolution 2: "Take Theirs" (User discards buffer and accepts external changes)
    {
        let reloaded_buffer = read_note(&note_path).unwrap();
        assert_eq!(reloaded_buffer, external_content);
    }

    // Test Resolution 3: "Keep Both / Fork" (User writes draft as conflict branch)
    {
        let conflict_branch_path = dir.path().join("conflict_note (User Copy).md");
        write_atomic(&conflict_branch_path, &in_memory_buffer).unwrap();

        assert_eq!(read_note(&note_path).unwrap(), external_content);
        assert_eq!(read_note(&conflict_branch_path).unwrap(), in_memory_buffer);
    }
}

#[test]
fn test_delete_to_trash_and_vault_fallback() {
    let dir = tempdir().expect("tempdir");
    let vault_root = dir.path();
    let target_file = vault_root.join("obsolete_note.md");

    write_atomic(&target_file, "Note to be deleted.").unwrap();
    assert!(target_file.exists());

    // Execute delete_to_trash
    let delete_res = delete_to_trash(vault_root, &target_file);
    assert!(
        delete_res.is_ok(),
        "delete_to_trash returned error: {:?}",
        delete_res
    );

    // Target file must no longer exist in original path
    assert!(
        !target_file.exists(),
        "Deleted file still exists at original path!"
    );

    // Either OS trash accepted it, or .trash/ fallback directory contains it
    let fallback_trash = vault_root.join(".trash");
    if fallback_trash.exists() {
        let entries: Vec<_> = fs::read_dir(&fallback_trash)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            entries.contains(&"obsolete_note.md".to_string()),
            "Fallback .trash missing deleted file: {:?}",
            entries
        );
    }
}

#[test]
fn test_import_collision_keep_both_policy() {
    let dir = tempdir().expect("tempdir");
    let vault = Vault::new(dir.path());

    // 1. Existing file in vault
    let existing_note = vault
        .create_note_with_collision_handling(Path::new(""), "Readme.md")
        .unwrap();
    write_atomic(&dir.path().join(&existing_note), "Original Vault Readme").unwrap();

    // 2. Import a new note with same base name under KeepBoth logic
    let duplicate_note = vault
        .create_note_with_collision_handling(Path::new(""), "Readme.md")
        .unwrap();
    write_atomic(&dir.path().join(&duplicate_note), "Imported New Readme").unwrap();

    // Verify both files exist with distinct names and unmodified contents
    assert_eq!(existing_note, PathBuf::from("Readme.md"));
    assert_eq!(duplicate_note, PathBuf::from("Readme 1.md"));

    let content_orig = read_note(&dir.path().join("Readme.md")).unwrap();
    let content_dupe = read_note(&dir.path().join("Readme 1.md")).unwrap();

    assert_eq!(content_orig, "Original Vault Readme");
    assert_eq!(content_dupe, "Imported New Readme");
}

#[test]
fn test_search_concurrency_during_active_file_modifications() {
    let dir = tempdir().expect("tempdir");
    let vault_root = dir.path().to_path_buf();

    // Seed vault with initial notes
    for i in 1..=20 {
        let note_path = vault_root.join(format!("Note_{:02}.md", i));
        let content = format!(
            "# Note {}\nContains search keyword ALPHA and unique payload {}.\n",
            i,
            i * 10
        );
        write_atomic(&note_path, &content).unwrap();
    }

    let running = Arc::new(AtomicBool::new(true));

    // Background thread 1: Actively mutates files in vault
    let running_writer = Arc::clone(&running);
    let writer_vault = vault_root.clone();
    let writer_handle = thread::spawn(move || {
        let mut counter = 100;
        while running_writer.load(Ordering::Relaxed) {
            let path = writer_vault.join(format!("Dynamic_{}.md", counter));
            let content = format!("Content with BETA keyword and count {}.\n", counter);
            let _ = write_atomic(&path, &content);
            counter += 1;
            thread::sleep(Duration::from_millis(5));
        }
    });

    // Main thread: Run quick_open and global searches repeatedly during writes
    for _ in 0..15 {
        // Collect current notes
        let note_paths: Vec<PathBuf> = fs::read_dir(&vault_root)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
            .map(|e| e.path().strip_prefix(&vault_root).unwrap().to_path_buf())
            .collect();

        // 1. Fuzzy quick open search
        let quick_results = quick_open_search(&note_paths, "Note");
        assert!(
            quick_results.len() >= 20,
            "Quick open search should locate at least 20 initial notes"
        );

        // 2. Global full text search for ALPHA
        let alpha_results = global_search(&vault_root, "ALPHA", &note_paths);
        assert_eq!(
            alpha_results.len(),
            20,
            "Global search should reliably find 20 ALPHA notes"
        );

        thread::sleep(Duration::from_millis(10));
    }

    // Stop writer thread
    running.store(false, Ordering::Relaxed);
    writer_handle.join().unwrap();
}
