## 2026-10-24T15:50:22Z
You are worker_m1_1.
Your working directory: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\worker_m1_1
Code workspace: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap
Authoritative User Request: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\ORIGINAL_REQUEST.md
Product Specification & Architecture: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\AGENT.md
Master Project Plan: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap\PROJECT.md
Requirements Catalog: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\spec_miner_survey_1\requirements.md

MANDATORY DIRECTIVES:
1. You MUST read ORIGINAL_REQUEST.md, AGENT.md, and PROJECT.md before starting work.
2. DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
3. Exclusive write ownership: You own `airgap/crates/**`, `airgap/Cargo.toml`, and `airgap/assets/**`. Do NOT write to `airgap/tests/`.

Scope for Milestone M1 (Workspace Skeleton, Core Vault Storage & Atomic I/O):
1. `crates/core` Data Safety & Storage Hardening:
   - Atomic writes in `io.rs`: implement safe temporary file writing (`write_all` + `flush` + `sync_all` / `fsync` before rename) ensuring no orphaned temp files on failure.
   - Line ending preservation (preserve CRLF if existing file uses CRLF; default LF for new notes) and UTF-8 BOM preservation/tolerance.
   - Path safety in `path_safety.rs`: check all Windows reserved names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`) including multi-extension variations (e.g. `con.bak.md`), illegal characters `<>:"/\|?*`, trailing dots/spaces, canonical vault root boundary enforcement, and symlink escape rejection.
   - Deletion & Recycle mechanism in `trash.rs`: move deleted notes/folders to OS recycle bin using the `trash` crate with automatic fallback to a timestamped `.trash/` folder inside the vault. Always require confirmation before deletion.
   - Vault model in `vault.rs`: natural sorting (folders first, alphabetical; notes alphabetical), `create_note` with collision suffixing (`Untitled.md`, `Untitled 1.md`), `create_folder`, `rename_item`, `delete_item`.
2. `crates/app` Desktop GUI & Layout:
   - Vault Picker: on launch, if no vault is configured or user requests, show an interactive vault selection screen (choose directory via OS dialog or pick/create a default sample vault).
   - Sidebar Tree: hierarchical folder and note navigation from disk (1:1 mapping), fold/unfold folders, active note selection, inline rename (`F2`), new note (`Ctrl+N`), new folder (`Ctrl+Shift+N`), delete confirmation modal.
   - Editor Pane: plain-text Markdown editor with line numbers, word wrap, cursor line highlight, undo/redo, and saved/unsaved status indicator.
   - Autosave: debounced autosave triggering ~1s after last keystroke, on window focus loss, and on application exit, writing atomically to disk.
   - Theme & Polish: custom dark and light themes plus follow-system, consistent spacing scale, clean typography (bundle/load clean fonts).
   - Layout: resizable sidebar divider, editor pane, bottom status bar showing word count, character count, and save state.
3. Verification:
   - `cargo test --workspace` must pass cleanly.
   - `cargo clippy --workspace --all-targets -- -D warnings` must pass with 0 warnings.
   - `cargo fmt --all -- --check` must pass with 0 diffs.
   - `cargo test --manifest-path tests/Cargo.toml` must pass all E2E tests.
4. Deliver your handoff report to `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\worker_m1_1\handoff.md`.
When finished, send a message back with the status and path to your handoff.
