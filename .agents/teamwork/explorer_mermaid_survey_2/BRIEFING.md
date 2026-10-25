# BRIEFING — 2026-10-24T15:05:00Z

## Mission
Investigate the Rust ecosystem for native Mermaid diagram rendering to de-risk M0: parsing, layout, SVG generation, resvg/usvg/tiny-skia rasterization, egui texture integration, theming, fallback, and zero-network verification.

## 🔒 My Identity
- Archetype: explorer
- Roles: investigation, synthesis, feasibility analysis
- Working directory: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_mermaid_survey_2
- Original parent: 85535336-9f0c-4c83-bb3f-dac37c864962
- Milestone: M0 (Mermaid Diagram Spike & Feasibility)

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Strict zero-network rule (no HTTP/network clients or runtime network access)
- Zero panics: parsing errors/unsupported syntax must render cleanly formatted code block with diagnostic message, wrapped in catch_unwind
- Pure native Rust rendering (no webview, headless browser, node.js, or JS runtime)

## Current Parent
- Conversation ID: 85535336-9f0c-4c83-bb3f-dac37c864962
- Updated: 2026-10-24T15:05:00Z

## Investigation State
- **Explored paths**:
  - `mermaid-rs-renderer` (v0.3.1) architecture, DAG layout via `dagre-rs` (Sugiyama algorithm), 23 diagram types, pure SVG generator.
  - `merman` (v0.9.2) headless Mermaid engine (used by Zed editor).
  - All 7 target diagram types: flowchart, sequence, class, state, ER, pie, gantt.
  - `resvg` + `usvg` + `tiny-skia` rasterization pipeline, fontdb configuration with bundled typography.
  - `egui` texture integration, HiDPI scale factor handling, content-hash LRU cache, and premultiplied alpha color image conversion.
  - Light/dark theme synchronization and transparent SVG background integration.
  - Defensive error handling: `catch_unwind`, execution timeouts, structured error diagnostics, and fallback UI code block with syntax highlighting.
  - Full dependency audit confirming zero network libraries and zero browser/JS runtimes.
- **Key findings**:
  - Native Mermaid rendering in Rust without any browser/webview is 100% feasible and ready for M0 implementation.
  - `mermaid-rs-renderer` achieves 2-6ms latency (500x-1400x faster than mermaid-cli).
  - Premultiplied alpha conversion pitfall identified and solved for `tiny-skia` -> `egui::ColorImage`.
- **Unexplored areas**: None for M0 scope.

## Key Decisions Made
- Recommended adopting `mermaid-rs-renderer` (v0.3.1, `default-features = false`) + `resvg` / `tiny-skia` in `crates/render`.
- Recommended off-UI-thread background worker with content-hash LRU caching and `catch_unwind` defensive fallback.

## Artifact Index
- DISPATCH.md — incoming dispatch instructions
- progress.md — liveness heartbeat and completed tasks
- mermaid_feasibility.md — comprehensive technical feasibility report
- handoff.md — 5-component handoff report
