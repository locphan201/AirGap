# BRIEFING — 2026-10-25T01:10:00Z

## Mission
Investigate and design concrete remediation specifications for Scroll Synchronization and Editor Conveniences wiring in `airgap/crates/app`.

## 🔒 My Identity
- Archetype: explorer
- Roles: Scroll & Editor Wire Explorer
- Working directory: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_4
- Original parent: 3afb76bf-62b1-4f73-84eb-b7f21076aae2
- Milestone: M2 Remediation

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Precise Rust specifications for egui integration
- Prevent infinite scroll loops between active driver and follower pane
- Provide exact function signatures, hook points, and code diff plan for the Worker

## Current Parent
- Conversation ID: 3afb76bf-62b1-4f73-84eb-b7f21076aae2
- Updated: not yet

## Investigation State
- **Explored paths**:
  - `airgap/crates/app/src/state.rs`: `ScrollSyncState`, `AppState`, `ViewMode`, `UndoManager`
  - `airgap/crates/app/src/ui/editor.rs`: `render_editor`, `handle_editor_keystrokes`, `apply_list_continuation`, `apply_indentation`, `apply_auto_pairing`, `find_current_line`
  - `airgap/crates/app/src/ui/preview.rs`: `render_preview`, `toggle_checkbox_at_line`
  - `airgap/crates/app/src/clipboard.rs`: `ClipboardHandler`, `try_paste_image`, `try_paste_smart_url`
  - `airgap/crates/app/src/app.rs`: Split-pane layout, execution order (`render_editor` then `render_preview`)
  - `egui-0.31.1` source: `ScrollArea` offset handling, `TextEdit` event dispatch loop, `TextEditState`, `CCursorRange`
- **Key findings**:
  1. `ScrollArea::vertical().vertical_scroll_offset(...)` directly sets `state.offset.y` when called. Follower pane can be driven without feedback loop by setting `vertical_scroll_offset` strictly on the follower and consuming via `.take()`.
  2. In `editor.rs`, keystrokes must be intercepted in a pre-process hook BEFORE `TextEdit::show(ui)` runs, because `TextEdit` destroys text selections and inserts raw characters/newlines during its internal event loop.
  3. Image paste in `handle_editor_keystrokes` must insert at active selection/cursor rather than appending to end of document (Finding 4).
  4. Task list toggle in `preview.rs` destroys CRLF endings (Finding 3); must detect existing newline delimiter (`\r\n` vs `\n`).
  5. `find_current_line` lacks UTF-8 boundary clamping (Finding 6); must use `floor_char_boundary`.
  6. Character index vs UTF-8 byte index mapping must be handled when bridging egui's `CCursor` and Rust `&str`.

## Key Decisions Made
- Use proportional scroll synchronization with one-way data flow: Driver pane (Editor or Preview) determines scroll ratio; follower pane applies `.vertical_scroll_offset(target)` consumed via `.take()`. Driver never receives programmatic scroll override, preventing any possibility of infinite loops.
- Use pre-execution event interception for `TextEdit`: Read `editor_char_range`, check events in `ui.input_mut`, apply modifications to `state.editor_content` and `TextEditState::cursor`, and consume the handled event (`retain`) so `TextEdit` renders the transformed state in the same frame without lag or conflict.

## Artifact Index
- DISPATCH.md — Dispatch log
- BRIEFING.md — Persistent context & state
- progress.md — Liveness heartbeat
- handoff.md — Final investigation report
