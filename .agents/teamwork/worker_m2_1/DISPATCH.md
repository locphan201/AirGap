## 2026-10-25T00:24:07Z
You are worker_m2_1 (Role: M2 Implementation Worker).
Your working directory is: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\worker_m2_1
Workspace root: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap

Mandatory files to read first:
- ORIGINAL_REQUEST.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\ORIGINAL_REQUEST.md
- AGENT.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\AGENT.md
- PROJECT.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap\PROJECT.md
- TEST_INFRA.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap\TEST_INFRA.md
- Explorer handoff reports:
  - C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_1\handoff.md
  - C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_2\handoff.md
  - C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_3\handoff.md

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Mission:
Implement Milestone M2 (Markdown Renderer, Code Highlighting & Live Preview) in `airgap`:
1. `airgap/crates/render`:
   - Configure dependencies: `pulldown-cmark = { version = "0.12", default-features = false, features = ["simd"] }` (with GFM options: tables, tasklists, strikethrough, footnotes, heading attributes), `syntect = { version = "5.2", default-features = false, features = ["regex-fancy", "default-syntaxes", "default-themes"] }`.
   - Update `crates/render/src/model.rs`: Full `DocModel` AST representing headings 1-6, paragraphs, code fences, blockquotes, lists, task lists (`- [ ]`, `- [x]`), GFM tables with column alignments, horizontal rules, inline formatting (bold, italic, strikethrough, code spans, links, local relative images), and source line mapping (`SourceSpan`).
   - Implement `crates/render/src/markdown.rs`: Parse markdown into `DocModel` with source line offsets.
   - Implement `crates/render/src/highlight.rs`: Syntax highlighting for code fences using `syntect` with pure-Rust `fancy-regex` backend; theme adaptation for Dark/Light palettes.
   - Implement `crates/render/src/image.rs`: Safe local image resolution relative to note path & vault root, rejecting directory traversal / symlink escapes / network URLs.
2. `airgap/crates/app`:
   - Implement Triple View Modes: Editor Only, Split View (50/50 resizable split), Preview Only (`Ctrl+\` cycle accelerator & toolbar button).
   - Implement Preview rendering in `crates/app/src/ui/preview.rs`: Render `DocModel` blocks, styled headings, paragraphs, formatted GFM tables, interactive task lists, fenced code blocks with language tag & copy button, local relative images, and external hyperlink handling (`open` crate with strict scheme whitelist: `http`, `https`, `mailto`).
   - Implement Editor-Preview scroll synchronization with feedback loop prevention.
   - Implement Editor interactive enhancements in `crates/app/src/ui/editor.rs`:
     - Line numbers gutter & active cursor line highlight.
     - Undo/redo stack.
     - In-note Find & Replace bar (`Ctrl+F` / `Ctrl+H` with case, whole-word, regex options).
     - Auto-pairing delimiters: `()`, `[]`, `{}`, `""`, ````.
     - List continuation on Enter (`- `, `* `, `1. `, `- [ ] `) & empty line backspace.
     - Tab / Shift+Tab 2-space indentation / outdent.
     - Smart URL paste: wrapping selection into `[selected](pasted_url)`.
     - Image paste: clipboard image saved to `assets/` and inserted as markdown image link.
     - Memoized / line-cached syntax highlighting in editor to avoid frame drops on 1MB notes.
3. Verification:
   - Write comprehensive unit and integration tests for all M2 components in `crates/render` and `crates/app`.
   - Run `cargo test --workspace` (all tests MUST pass, including existing M0/M1 and E2E tests).
   - Run `cargo clippy --workspace --all-targets -- -D warnings` (MUST be 100% clean).
   - Run `cargo fmt --all -- --check` (MUST be 100% clean).

Deliver your completion report in `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\worker_m2_1\handoff.md` and send a message back when finished.
