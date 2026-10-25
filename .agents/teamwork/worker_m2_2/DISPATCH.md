## 2026-10-25T01:31:33Z
You are worker_m2_2 (Role: M2 Remediation Worker).
Your working directory is: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\worker_m2_2
Workspace root: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap

Mandatory files to read first:
- ORIGINAL_REQUEST.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\ORIGINAL_REQUEST.md
- AGENT.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\AGENT.md
- PROJECT.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap\PROJECT.md
- TEST_INFRA.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap\TEST_INFRA.md
- Reviewer & Challenger defect reports:
  - C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\reviewer_m2_1\handoff.md
  - C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\challenger_m2_1\handoff.md
- Explorer remediation blueprints & concrete code patches:
  - C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_4\handoff.md
  - C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_5\handoff.md
  - C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_6\handoff.md

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Mission:
Execute full remediation of all Milestone M2 defects and gaps across `airgap/crates/render` and `airgap/crates/app`:
1. Genuine Scroll Synchronization:
   - Wire `ScrollSyncState` to `ScrollArea::vertical().vertical_scroll_offset(...)` in both `editor.rs` and `preview.rs`.
   - Actuate real scroll offsets and consume via `.take()`, preventing circular feedback loops.
2. Wire Editor Conveniences directly into `egui::TextEdit` / event loop in `editor.rs`:
   - Intercept `Enter` to run list/task continuation (continue prefix, clear empty bullet).
   - Intercept `Tab` and `Shift+Tab` to run 2-space indent/outdent.
   - Intercept auto-pairing characters (`(`, `[`, `{`, `"`, ````) to insert pair or wrap selection.
   - Intercept `Ctrl+V` to run smart URL paste (`[selected](url)`) when text is selected and clipboard contains a valid URL.
3. Fix UTF-8 Character Boundary Panic:
   - In `editor_highlighter.rs` (`append_spans_with_find`), clamp offsets and guard with `text.is_char_boundary()` and monotonic progression to prevent panics on CJK/multibyte text when typing before find matches.
4. Implement Two-Tier Line-Level Differential Caching:
   - In `editor_highlighter.rs`, implement line-chunked memoization to bring 1MB note keystroke latency to <15ms in release mode.
5. Fix Checkbox CRLF Line Ending Preservation:
   - In `preview.rs` (`toggle_checkbox_at_line`), preserve original line ending delimiters (`\r\n` vs `\n`) when toggling task checkboxes.
6. Cursor-Aware Image Paste:
   - In `editor.rs`, insert pasted image Markdown at the active cursor/selection position rather than appending to end of file.
7. Auto-Generated Heading Slug IDs:
   - In `markdown.rs`, implement `generate_slug` and populate `id` when heading attribute id is `None`.
8. Guard Character Boundaries in `find_current_line`:
   - In `editor.rs`, use `text.floor_char_boundary(...)` to prevent byte index panics.
9. Verification:
   - Run `cargo test --manifest-path airgap/Cargo.toml --workspace`
   - Run `cargo test --manifest-path airgap/Cargo.toml --test challenger_m2_adversarial`
   - Run `cargo test --manifest-path airgap/Cargo.toml --test challenger_m2_security_adversarial`
   - Run `cargo test --manifest-path airgap/Cargo.toml --test test_m2_integration`
   - Run `cargo clippy --manifest-path airgap/Cargo.toml --workspace --all-targets -- -D warnings`
   - Run `cargo fmt --manifest-path airgap/Cargo.toml --all -- --check`
   - Verify all tests pass with zero warnings and zero diffs.

Deliver your completion report in `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\worker_m2_2\handoff.md` and send a message back when finished.
