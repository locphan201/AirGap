# Handoff Report — Specification & Feature Inventory Mining

**Agent ID:** spec_miner_survey_1  
**Target Milestone:** Milestone 0 / Survey Phase  
**Artifact Path:** `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\spec_miner_survey_1\requirements.md`  
**Date:** 2026-10-24T15:02:00Z  

---

## 1. Observation

1. **Source Documents Examined**:
   - `ORIGINAL_REQUEST.md` (lines 1–48): Defines requirements R1–R6, the three-crate workspace (`core`, `render`, `app`), acceptance criteria, test suite targets, performance bounds (1 MB note, <150 MB idle RAM), and zero-network mandate.
   - `AGENT.md` (lines 1–158): Provides comprehensive architecture details, Section 1 through Section 13, defining fixed design decisions, editor/preview/vault capabilities, Mermaid native rendering requirements, 8 open technical decisions, 10 non-goals, and milestone gates (M0–M6).
2. **Workspace Status**:
   - Inspected `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap`: currently an empty directory ready for project workspace scaffolding.
   - Verified that no prior Rust code exists in `airgap`.
3. **Extracted Specifications**:
   - Identified **77 distinct functional and architecture features** categorized across 12 functional domains:
     - Multi-Crate Architecture (`ARCH`): 3 features (Features 1–3)
     - Vault & Directory Operations (`VAULT`): 8 features (Features 4–11)
     - Data Safety & Integrity (`SAFE`): 10 features (Features 12–21)
     - Markdown Editor (`EDIT`): 14 features (Features 22–35)
     - Markdown Preview (`PREV`): 8 features (Features 36–43)
     - Native Mermaid Diagram Engine (`MERM`): 12 features (Features 44–55)
     - Search & Navigation (`SRCH`): 4 features (Features 56–59)
     - Import Pipelines (`IMP`): 5 features (Features 60–64)
     - Export Pipelines (`EXP`): 3 features (Features 65–67)
     - UI/UX, Theming & Accessibility (`UIUX`): 7 features (Features 68–74)
     - Privacy & Local-Only Guarantees (`PRIV`): 1 feature (Feature 75)
     - Distribution & Packaging (`DIST`): 2 features (Features 76–77)
   - Identified **34 critical edge cases** covering Windows reserved device names, path length limits, illegal characters, line ending preservation, UTF-8 BOM, symlinks, filesystem conflict handling, zip-slip, zip-bomb, Mermaid parse errors, and UI frame rates.
   - Catalogued **8 open technical questions/spikes** (Mermaid crate evaluation, TextEdit vs code editor widget, preview pipeline, tab bar vs single document, syntax highlighter engine, HTML-to-MD parser, PDF deferral, and crate naming).
   - Documented **10 explicit non-goals** (cloud sync, user accounts, collaboration, plugins, backlinks graph, mobile apps, WYSIWYG, spellcheck, LaTeX, and external databases).

---

## 2. Logic Chain

1. **Step 1 (Source Extraction)**: `ORIGINAL_REQUEST.md` (lines 15–32) mandates multi-crate partitioning, atomic writes, debounced watcher, syntax-highlighted editor, CommonMark+GFM preview, native Mermaid rendering, fuzzy quick open, global search, safe import/export, and zero network calls.
2. **Step 2 (Architectural Elaboration)**: `AGENT.md` (Sections 2–6) expands these into concrete component boundaries:
   - `core` must be completely UI-agnostic and handle path normalization, atomic writes, trash fallback, notify watcher, search, and zip-slip/zip-bomb guards.
   - `render` parses Markdown and translates Mermaid to SVG and rasterizes to RGBA pixmaps via `resvg`/`usvg`/`tiny-skia` with caching.
   - `app` implements `egui`/`eframe` GUI with async worker channels off the main thread.
3. **Step 3 (Safety & Platform Specifics)**: `AGENT.md` Section 6 details mandatory platform edge cases: Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`), trailing dots/spaces, CRLF/LF preservation, UTF-8 BOM tolerance, and 260-char path limits. These dictate core validation rules.
4. **Step 4 (Mermaid Engine Isolation)**: `AGENT.md` Section 4 designates Mermaid rendering as the highest-risk item (Gate M0) requiring native Rust SVG rasterization without webviews, covering flowchart, sequence, class, state, ER, pie, and gantt, with graceful error fallback.
5. **Step 5 (Synthesis)**: Synthesizing Steps 1–4 yields the complete, prioritized feature matrix and edge cases document in `requirements.md`, establishing the authoritative requirements baseline for the orchestrator and implementation subagents.

---

## 3. Caveats

- **Crate Availability Spike**: While `mermaid-rs-renderer` and layout alternatives are noted in the specification, actual validation of their crates.io version, maintenance status, and rendering fidelity is delegated to `explorer_mermaid_survey_2`.
- **GUI Editor Widget Selection**: Determining whether `egui::TextEdit::multiline` with a custom layouter achieves acceptable 60 FPS performance on 1 MB notes with CJK IME input is an empirical question to be validated during Milestone 0/1 spikes.
- **Operating System Scope**: First-class support is specified for Windows 10/11 and Linux (X11 & Wayland). macOS compilation is preserved but not packaged.

---

## 4. Conclusion

The specification mining and feature inventory extraction for "AirGap" is complete. Every functional requirement (R1–R6), architectural boundary, non-functional budget, edge case condition, and acceptance criterion has been catalogued in `requirements.md`. The requirements document provides an unambiguous, traceable foundation for Milestone 0 and subsequent implementation phases.

---

## 5. Verification Method

1. **Inspect Requirements Document**:
   - View `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\spec_miner_survey_1\requirements.md`.
   - Confirm all 77 features are categorized with inputs, outputs, error behaviors, priorities, and verification criteria.
   - Confirm all 34 edge cases are documented with mitigation strategies.
   - Confirm all 8 open items and 10 non-goals are catalogued.
2. **Cross-Check with Source Documents**:
   - Compare `requirements.md` against `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\ORIGINAL_REQUEST.md` and `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\AGENT.md`.
3. **Invalidation Conditions**:
   - The findings would be invalidated if requirements R1–R6 or AGENT.md Sections 1–13 contain mandatory features, edge cases, or non-functional criteria that are omitted from `requirements.md`.
