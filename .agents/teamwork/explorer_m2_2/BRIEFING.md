# BRIEFING — 2026-10-25T00:22:45Z

## Mission
Investigate and design syntax highlighting and editor styling pipeline for `airgap` (syntect with fancy-regex, theme-aware syntax colors, preview fenced code blocks with copy button, and high-performance live editor highlighting in `egui::TextEdit`).

## 🔒 My Identity
- Archetype: explorer
- Roles: Syntax Highlighting Explorer
- Working directory: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_2
- Original parent: 3afb76bf-62b1-4f73-84eb-b7f21076aae2
- Milestone: M2

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Syntect pure-Rust backend (`fancy-regex` / `default-fancy`, no onig / C/C++ regex dependencies)
- Theme-aware syntax colors (Dark / Light / System egui::Color32 mapping)
- High performance editor styling for large notes (prevent 130ms frame drops on 1MB notes via memoization / line-based caching)
- Preview fenced code block rendering with rounded corners, background, and copy button

## Current Parent
- Conversation ID: 3afb76bf-62b1-4f73-84eb-b7f21076aae2
- Updated: not yet

## Investigation State
- **Explored paths**:
  - `airgap/crates/render/Cargo.toml` and `Cargo.lock` (syntect 5.3.0 with fancy-regex, zero onig)
  - `airgap/crates/render/src/highlight.rs` (SyntaxHighlighter, HighlightedSpan)
  - `airgap/crates/render/src/model.rs` and `markdown.rs` (Block::CodeBlock)
  - `airgap/crates/app/src/ui/editor.rs` (current un-styled TextEdit)
  - `airgap/crates/app/src/theme.rs` (ThemeMode, Visuals)
  - `airgap/crates/app/tests/adversarial_editor_stress.rs` (1MB benchmark measurements)
- **Key findings**:
  1. Syntect configuration is 100% pure-Rust via `fancy-regex 0.16.2` without C/C++ Oniguruma dependencies.
  2. Syntect static OnceLock optimization is required to prevent repeating expensive 5MB syntax set deserialization per frame.
  3. Editor styling should use a fast line-level Markdown lexer with memoization (`EditorHighlighter`) returning cached `Arc<Galley>` in 0.00ms when idle and diffing only edited lines (<5ms typing), avoiding 130ms frame freezes.
  4. Preview fenced code blocks use syntect for 100+ programming languages, single `LayoutJob` per block for crisp rendering, custom frame with rounded corners, and native egui clipboard copy button.
- **Unexplored areas**: None for M2 syntax highlighting scope.

## Key Decisions Made
- Architecture decoupled: `airgap-render` provides UI-agnostic `SyntaxHighlighter` with RGB spans and static OnceLock; `airgap-app` provides `EditorHighlighter` for live markdown layout and `render_code_block` for preview.
- Theme mapping: `base16-ocean.dark` for Dark mode, `InspiredGitHub` / `base16-ocean.light` for Light mode, with full WCAG AA contrast compliance.

## Artifact Index
- DISPATCH.md — incoming dispatch instructions
- BRIEFING.md — persistent situational awareness
- progress.md — liveness heartbeat
- handoff.md — final comprehensive handoff report
