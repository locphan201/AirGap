# BRIEFING — 2026-10-25T01:11:30Z

## Mission
Investigate and produce concrete remediation specifications for CRLF line ending preservation, image paste cursor positioning, auto-generated heading slug IDs, and UTF-8 char boundary safety in editor/preview/render crates.

## 🔒 My Identity
- Archetype: explorer
- Roles: Editor Formatting & Image Explorer
- Working directory: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_6
- Original parent: 3afb76bf-62b1-4f73-84eb-b7f21076aae2
- Milestone: M2 remediation

## 🔒 Key Constraints
- Read-only investigation — do NOT implement directly in source code
- Write only to working directory: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_6
- Produce concrete remediation specifications for the 4 identified issues
- Write handoff report following 5-Component structure in handoff.md

## Current Parent
- Conversation ID: 3afb76bf-62b1-4f73-84eb-b7f21076aae2
- Updated: not yet

## Investigation State
- **Explored paths**:
  - `airgap/crates/app/src/ui/preview.rs`: `toggle_checkbox_at_line` (lines 754–773)
  - `airgap/crates/app/src/ui/editor.rs`: `render_editor` and `handle_editor_keystrokes` (lines 257, 457–466), `find_current_line` (lines 644–655)
  - `airgap/crates/render/src/markdown.rs`: `ActiveHeading`, `Event::Start(Tag::Heading)` and `Event::End(TagEnd::Heading)` (lines 222–257)
  - `airgap/crates/render/src/lib.rs`: exports
  - `airgap/crates/app/src/clipboard.rs`: `ClipboardHandler::try_paste_image` and `try_paste_smart_url`
  - `egui-0.31.1`: `CursorRange`, `TextEditState`, `TextEditOutput`, `char_indices` offset mapping
- **Key findings**:
  1. `toggle_checkbox_at_line` uses `lines().join("\n")`, discarding `\r\n` and trailing newlines. Remediation uses `split_inclusive('\n')`.
  2. `try_paste_image` appends to `state.editor_content` via `push_str`. Remediation passes `output: &TextEditOutput`, converts cursor char indices to byte offsets, uses `replace_range`, and updates `TextEditState`.
  3. Headings without explicit `{#id}` have `id: None`. Remediation adds `generate_slug` and populates `id` when `None`.
  4. `find_current_line` slices with `clamped_cursor = cursor_char_idx.min(text.len())`, panicking on non-char boundaries. Remediation uses `text.floor_char_boundary(clamped_cursor)`.
- **Unexplored areas**: None for the assigned 4 scope items.

## Key Decisions Made
- Chose `split_inclusive('\n')` for Issue 1 for in-place streaming preservation of per-line delimiters and trailing newlines.
- Designed `char_idx_to_byte_idx` helper for Issue 2 to bridge egui `CCursor` character indices with Rust `&str` byte ranges safely.
- Designed `generate_slug` for Issue 3 adhering to CommonMark/GitHub anchor conventions with Unicode CJK support.
- Chose `text.floor_char_boundary` for Issue 4 to guarantee UTF-8 boundary safety across arbitrary cursor offsets.

## Artifact Index
- DISPATCH.md — incoming dispatch record
- BRIEFING.md — persistent working memory
- progress.md — liveness heartbeat
- proposed_preview_fix.patch — machine-applicable diff for preview.rs
- proposed_editor_fix.patch — machine-applicable diff for editor.rs
- proposed_render_fix.patch — machine-applicable diff for markdown.rs and lib.rs
- handoff.md — final handoff report
