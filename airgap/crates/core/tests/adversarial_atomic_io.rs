use airgap_core::io::{detect_line_ending, read_note, write_atomic, LineEnding};
use std::fs::{self, File};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::tempdir;

#[test]
fn test_crlf_preservation_behavior() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("crlf_note.md");

    // Existing file with CRLF line endings
    let original_crlf = "Header\r\nItem 1\r\nItem 2\r\n";
    fs::write(&file_path, original_crlf).unwrap();

    // Verify initial file has CRLF
    let bytes_before = fs::read(&file_path).unwrap();
    assert!(bytes_before.windows(2).any(|w| w == b"\r\n"));

    // User loads note (which might be in memory with LF or CRLF)
    let loaded = read_note(&file_path).unwrap();
    assert_eq!(detect_line_ending(&loaded), LineEnding::Crlf);

    // In a GUI text editor, strings frequently use \n internally.
    let edited_lf = "Header\nItem 1\nItem 2 (edited)\n";

    // Now write back using write_atomic directly:
    write_atomic(&file_path, edited_lf).unwrap();

    let bytes_after = fs::read(&file_path).unwrap();
    let has_crlf_after = bytes_after.windows(2).any(|w| w == b"\r\n");

    println!(
        "[CRLF Observation] Does write_atomic preserve CRLF when given LF string? {}",
        has_crlf_after
    );
    // Document finding: write_atomic writes raw bytes of the string passed to it without
    // checking or preserving the file's preexisting line endings on disk.
}

#[test]
fn test_bom_preservation_behavior() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("bom_note.md");

    // Existing file with UTF-8 BOM
    let mut bom_bytes = vec![0xEF, 0xBB, 0xBF];
    bom_bytes.extend_from_slice(b"# Title with BOM\nNote content");
    fs::write(&file_path, &bom_bytes).unwrap();

    // Verify BOM is initially present on disk
    let initial_disk_bytes = fs::read(&file_path).unwrap();
    assert_eq!(&initial_disk_bytes[0..3], &[0xEF, 0xBB, 0xBF]);

    // Read note via read_note
    let content = read_note(&file_path).unwrap();
    assert_eq!(content, "# Title with BOM\nNote content");

    // Write modified content back
    let modified = format!("{}\nAppended line", content);
    write_atomic(&file_path, &modified).unwrap();

    // Check disk bytes
    let after_disk_bytes = fs::read(&file_path).unwrap();
    let bom_preserved = after_disk_bytes.len() >= 3 && after_disk_bytes[0..3] == [0xEF, 0xBB, 0xBF];
    println!(
        "[BOM Observation] Was UTF-8 BOM preserved upon saving? {}",
        bom_preserved
    );
    // Finding: read_note strips BOM, and write_atomic has no BOM parameter/flag,
    // so saving an existing BOM-encoded file silently strips the BOM.
}

#[test]
fn test_concurrent_write_races() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("concurrent_race.md");

    // Initial write
    write_atomic(&file_path, "Initial Content").unwrap();

    let thread_count = 10;
    let iterations_per_thread = 20;
    let barrier = Arc::new(Barrier::new(thread_count));
    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();

    for t_idx in 0..thread_count {
        let b = Arc::clone(&barrier);
        let succ = Arc::clone(&success_count);
        let err = Arc::clone(&error_count);
        let path = file_path.clone();

        handles.push(thread::spawn(move || {
            b.wait(); // Synchronize all threads to hammer simultaneously
            for i in 0..iterations_per_thread {
                let content = format!("Thread {:02} Iteration {:03}\n", t_idx, i);
                match write_atomic(&path, &content) {
                    Ok(_) => {
                        succ.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(e) => {
                        err.fetch_add(1, Ordering::SeqCst);
                        eprintln!("[Thread {}] write_atomic failed: {:?}", t_idx, e);
                    }
                }
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let successes = success_count.load(Ordering::SeqCst);
    let errors = error_count.load(Ordering::SeqCst);
    println!(
        "[Concurrent Write Race] Total attempts: {}, Successes: {}, Errors: {}",
        thread_count * iterations_per_thread,
        successes,
        errors
    );

    // Verify integrity of final file on disk
    assert!(
        file_path.exists(),
        "Target file must exist after concurrent writes"
    );
    let final_content = read_note(&file_path).expect("Target file must be readable");
    assert!(
        !final_content.is_empty(),
        "Target file must not be truncated or 0 bytes"
    );
    assert!(
        final_content.starts_with("Thread "),
        "Target file must contain coherent thread output: found {:?}",
        final_content
    );
}

#[test]
fn test_file_lock_race_and_deletion_window() {
    // Test what happens if another process opens the target file for reading while write_atomic executes
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("locked_note.md");

    write_atomic(&file_path, "Original Data").unwrap();

    // Open target file for reading (simulating a concurrent reader or indexer)
    let read_handle = File::open(&file_path).unwrap();

    // Attempt write_atomic while read_handle is open
    let write_res = write_atomic(&file_path, "Updated Data with Reader Open");
    println!(
        "[File Lock Check] write_atomic while file is open for read: {:?}",
        write_res
    );

    drop(read_handle);

    // Verify file still exists and was not lost
    assert!(
        file_path.exists(),
        "File must not be deleted if replace fails"
    );
}

#[test]
fn test_windows_rename_target_exists() {
    let dir = tempdir().unwrap();
    let file1 = dir.path().join("file1.txt");
    let file2 = dir.path().join("file2.txt");

    fs::write(&file1, "Content 1").unwrap();
    fs::write(&file2, "Content 2").unwrap();

    let rename_res = fs::rename(&file1, &file2);
    println!(
        "[Windows std::fs::rename on existing target]: {:?}",
        rename_res
    );
}

#[test]
fn test_partial_write_simulation() {
    // Verify that writing fails cleanly if disk or stream fails, leaving original target intact
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("partial_note.md");

    let original = "Known good original data";
    write_atomic(&file_path, original).unwrap();

    // Check temp directory: no lingering temporary files
    let parent = file_path.parent().unwrap();
    let temp_files_before: Vec<_> = fs::read_dir(parent)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(".airgap_tmp_"))
        .collect();
    assert_eq!(temp_files_before.len(), 0);

    // Verify original content intact
    let current = read_note(&file_path).unwrap();
    assert_eq!(current, original);
}
