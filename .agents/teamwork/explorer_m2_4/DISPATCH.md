## 2026-10-25T01:04:04Z
You are explorer_m2_4 (Role: Scroll & Editor Wire Explorer).
Your working directory is: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_4
Workspace root: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap

Mandatory files to read:
- ORIGINAL_REQUEST.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\ORIGINAL_REQUEST.md
- AGENT.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\AGENT.md
- PROJECT.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap\PROJECT.md
- Reviewer M2 report: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\reviewer_m2_1\handoff.md
- Code files: `airgap/crates/app/src/ui/editor.rs`, `airgap/crates/app/src/ui/preview.rs`, `airgap/crates/app/src/state.rs`, `airgap/crates/app/src/clipboard.rs`

Mission:
Investigate and produce concrete remediation specifications for:
1. Genuine Scroll Synchronization between Editor and Preview panels in `airgap/crates/app`:
   - Design how `egui::ScrollArea::vertical().vertical_scroll_offset(...)` or `ui.scroll_to_rect(...)` must be actuated.
   - Eliminate dead fields in `ScrollSyncState`.
   - Prevent infinite scroll loops between the active driver and follower pane.
2. Wire Editor Conveniences directly into `egui::TextEdit` / event loop in `editor.rs`:
   - Intercept Enter key to execute `apply_list_continuation` (continue prefixes, handle empty bullets).
   - Intercept Tab and Shift+Tab keys to execute `apply_indentation` (2 spaces indent/outdent).
   - Intercept auto-pairing keys `(`, `[`, `{`, `"`, ```` to insert pairs or wrap selection.
   - Intercept Ctrl+V to invoke `ClipboardHandler::try_paste_smart_url` when text is selected and clipboard contains a valid URL.
3. Provide exact function signatures, hook points, and code diff plan for the Worker.

Write your report to `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_4\handoff.md` and send a message back when done.
