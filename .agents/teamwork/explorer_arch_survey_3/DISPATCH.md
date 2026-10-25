## 2026-10-24T14:57:40Z
You are explorer_arch_survey_3.
Your working directory: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_arch_survey_3
Authoritative User Request: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\ORIGINAL_REQUEST.md
Product Specification & Architecture: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\AGENT.md

MANDATORY: You MUST read ORIGINAL_REQUEST.md and AGENT.md before starting work.
Your task is to design the Cargo workspace architecture, crate boundaries, data structures, and core algorithms for "AirGap".

Specifically investigate and design:
1. Cargo Workspace layout (airgap/ root):
   - crates/core: Vault model, tree hierarchy, atomic write (tempfile + sync + rename), path sanitization (traversal rejection, Windows reserved names CON/PRN/AUX/NUL/COM1-9/LPT1-9, long paths), deletion & trash (trash crate + .trash/ fallback), filesystem watcher (notify with debouncing), background search (nucleo/fuzzy-matcher for quick open, multi-threaded text search), import (folder, Obsidian, zip with zip-slip / zip-bomb guards), export (zip, HTML). No UI dependencies!
   - crates/render: Markdown AST / parser (pulldown-cmark), syntax highlighting (syntect with pure-Rust regex or lightweight alternative), Mermaid SVG generation & rasterization (resvg, tiny-skia), document model.
   - crates/app: egui + eframe desktop application, state management, background task channels (crossbeam-channel), UI layout (sidebar, editor with syntax highlighting and line numbers, preview with split/modes and scroll sync, search modal, conflict dialog).
2. Concurrency and Non-Blocking UI Model:
   - Message passing between background worker threads and egui UI loop (ctx.request_repaint()).
   - Conflict resolution state machine (when external watcher reports file modification while unsaved edits exist).
3. Privacy & Dependency Audit:
   - Verify all chosen crates have no transitive network/HTTP dependencies (reqwest, curl, hyper, ureq, tokio network features).
4. Release profile settings: lto = "fat", codegen-units = 1, strip = true, panic = "abort".
5. Write your complete design document to C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_arch_survey_3\architecture_plan.md and summary handoff in C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_arch_survey_3\handoff.md. Update your progress.md regularly.
When finished, send a message back with the status and path to your handoff.
