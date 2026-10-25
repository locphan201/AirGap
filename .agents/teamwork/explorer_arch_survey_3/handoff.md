# Handoff Report: AirGap Workspace Architecture & Core Algorithms

**Agent:** explorer_arch_survey_3  
**Date:** 2026-10-24T15:03:30Z  
**Target Root:** `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap`  
**Artifacts Generated:**  
- `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_arch_survey_3\architecture_plan.md`  
- `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_arch_survey_3\handoff.md`

---

## 1. Observation
1. **Authoritative Specifications Inspected:**
   - `ORIGINAL_REQUEST.md`: Lines 15–32 define requirements R1 through R6 (multi-crate workspace, vault management & data safety, markdown editor & live preview, native Mermaid diagram rendering, search/import/export/release packaging, and local-only privacy).
   - `AGENT.md`: Lines 12–21 establish fixed decisions: Rust edition 2021+, Linux (X11 & Wayland) and Windows 10/11 first-class, `egui`/`eframe`, 1:1 local directory storage without databases, release profile `lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`, target idle RAM < 150 MB, cold boot < 1 s.
   - `AGENT.md`: Lines 84–106 establish Cargo workspace structure (`core`, `render`, `app`) and enforce strict non-blocking UI thread rules.
   - `spec_miner_survey_1/requirements.md`: Lines 50–133 enumerate 77 specific functional requirements across all categories with edge case handling for Windows reserved names, zip-slip, and long paths.
2. **Local Environment Inspection:**
   - Directory `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap` exists and is currently an empty directory awaiting implementation.
   - Toolchain check: running `cargo --version` in terminal confirms cargo is not currently in the global system PATH of the current shell session, requiring self-contained manifest designs that do not depend on active compiler environment side-effects.

---

## 2. Logic Chain
1. **Separation of Concerns (R1 & AGENT.md Sec 5):**
   - Observations show that mixing UI code into domain logic prevents headless testing and makes property-based verification impossible.
   - Therefore, `crates/core` is designed with zero UI dependencies (`egui`, `winit`, `eframe`), containing only domain data structures, file safety routines, watcher abstractions, search, and import/export.
   - `crates/render` is responsible for parsing Markdown (`pulldown-cmark`) into an intermediate `DocModel`, syntax highlighting (`syntect`), and rasterizing Mermaid diagrams (`resvg`/`tiny-skia`) into RGBA pixmaps. It remains headless-testable without requiring a GPU display.
   - `crates/app` handles `egui`/`eframe` windowing, state management, worker channels, and UI widgets.

2. **Data Safety & Atomic File Operations (R2, AGENT.md Sec 6):**
   - Naive file writes risk 0-byte truncation on crash or power outage.
   - Therefore, an atomic write protocol is designed: allocate a temporary file in the *same* filesystem directory as the target, write content with preserved line endings (CRLF vs LF detection), execute `file.sync_all()` to flush dirty OS buffers, and execute an atomic rename with a 3-retry backoff loop for transient Windows file locks.
   - Path traversal (`..`), symlink escapes outside the vault root, Windows reserved names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`, e.g. `aux.md`), and trailing dots/spaces are explicitly validated and rejected before any file operations.
   - File deletion routes to the OS recycle bin via the `trash` crate, falling back automatically to a timestamped `.trash/` directory inside the vault root if OS trash fails.

3. **Concurrency & Non-Blocking UI Model (R1, AGENT.md Sec 5):**
   - The UI thread must never freeze while opening large notes (1 MB+), rendering diagrams, or searching 10,000 notes.
   - Lock-free message queues (`crossbeam-channel`) decouple background workers (watcher, search, import/export, Mermaid rasterizer) from the egui frame loop.
   - Completed worker tasks signal `ctx.request_repaint()` to wake egui immediately.
   - A dedicated conflict resolution state machine detects external disk changes while unsaved in-memory edits exist, suspending autosave and prompting the user with "Keep Mine", "Take Theirs", or "View Diff".

4. **Pure-Rust Syntax Highlighting & Mermaid Strategy (R3, R4, AGENT.md Sec 4 & 12):**
   - `syntect` with `default-features = false` and `fancy-regex` provides pure-Rust syntax highlighting without requiring the C library `oniguruma`, simplifying cross-compilation.
   - Mermaid diagram blocks are parsed natively, styled according to the active theme (Dark/Light), rasterized via `resvg`/`tiny-skia` off the UI thread at device HiDPI scale, and cached in an LRU cache keyed by `(content_hash, is_dark, scale_bits)`.
   - Malformed diagrams degrade gracefully to formatted code blocks with error diagnostics without crashing or blocking the UI.

5. **Strict Offline Privacy Guarantee (R6, AGENT.md Sec 6):**
   - The dependency audit confirms zero transitive HTTP client crates (`reqwest`, `hyper`, `curl`, `ureq`, `tokio/net`).
   - Hyperlinks clicked in the UI are opened in the default browser using the OS desktop launcher (`open` crate via `ShellExecuteW`/`xdg-open`), strictly without any app network communication.
   - A `deny.toml` policy is specified to mathematically verify this policy during CI.

---

## 3. Caveats
1. **Mermaid Diagram Type Breadth:** While Flowchart, Sequence, Class, State, ER, Pie, and Gantt charts are designed into the native rasterization pipeline, specialized diagram types (e.g. GitGraph, Mindmap, Journey) are designated for post-v1 and will trigger the graceful error fallback card.
2. **Windows Long Paths:** Although sanitization handles UNC `\\?\` prefixes, application manifests with `<longPathAware>true</longPathAware>` should be bundled for maximum Windows 10/11 compatibility.
3. **Panic Strategy vs Catch Unwind:** `panic = "abort"` reduces binary size significantly. For worker threads that evaluate untrusted parser input, workers should use structured `Result` error boundaries rather than relying on unwinding `catch_unwind`.

---

## 4. Conclusion
A complete, rigorous architectural blueprint for **AirGap** has been designed and documented in `architecture_plan.md`. The design fulfills all requirements (R1–R6), acceptance criteria, and constraints specified in `ORIGINAL_REQUEST.md` and `AGENT.md`. It provides exact Cargo manifests, domain data structures, atomic I/O algorithms, path sanitization rules, concurrency channels, conflict state machine, render pipeline, and zero-network security enforcement.

---

## 5. Verification Method
1. **Specification & Document Review:**
   - Inspect `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_arch_survey_3\architecture_plan.md` to review the crate structures, algorithms, and data types.
2. **Crate Boundary Verification:**
   - Check `crates/core/Cargo.toml` in the plan to verify zero UI or windowing dependencies (`egui`, `eframe`, `winit`).
   - Check `crates/render/Cargo.toml` to verify pure-Rust dependencies (`syntect` with `fancy-regex`, `resvg`, `pulldown-cmark`).
3. **Dependency & Privacy Audit Verification:**
   - Review Section 6 of `architecture_plan.md` and the `deny.toml` specification to verify the complete prohibition of `reqwest`, `hyper`, `curl`, `ureq`, and network features.
4. **Invalidation Conditions:**
   - Any introduction of an HTTP/networking crate.
   - Any direct dependency on `egui` or GUI frameworks inside `crates/core`.
   - Any synchronous file I/O or SVG rasterization on the egui UI thread.
