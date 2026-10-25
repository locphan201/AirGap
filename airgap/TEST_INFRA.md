# AirGap — End-to-End (E2E) Test Infrastructure Specification

**Document Version:** 1.0.0  
**Status:** Approved & Implemented  
**Workspace Root:** `airgap/`  
**Test Suite Directory:** `airgap/tests/`  
**Applicable Crates:** `airgap-core`, `airgap-render`, `airgap-app`  

---

## 1. Executive Summary & Infrastructure Overview

AirGap is a native, local-only Markdown desktop application engineered in Rust for Windows 10/11 and Linux (X11/Wayland). It operates under strict non-negotiable constraints:
1. **Zero-Network Privacy:** Complete absence of network libraries, sockets, telemetry, or external web runtimes.
2. **Plain-File Vaults:** Notes are stored 1:1 on disk as standard `.md` files without proprietary databases.
3. **Native Non-Web Rendering:** CommonMark/GFM and Mermaid diagrams are parsed and rendered directly in Rust without Chromium, Electron, WebViews, or Node.js.
4. **Ironclad Data Safety:** Atomic write protocols, path traversal blocks, Windows reserved name sanitization, trash fallbacks, and collision safeguards.

To ensure that these invariants are never violated, the **AirGap E2E Test Infrastructure** establishes an opaque-box, multi-tiered test suite located in `airgap/tests/`. This document defines the architectural philosophy, complete feature inventory mapping, 4-tier testing methodology, real-world application scenarios, and step-by-step test execution protocols.

---

## 2. Core Testing Philosophy

### 2.1 Opaque-Box & Requirement-Driven Architecture
Every test in this suite is written against the public interface contracts defined in `PROJECT.md` and requirements specified in `ORIGINAL_REQUEST.md` and `AGENT.md`. Tests interact with the system strictly through exposed crate interfaces, filesystem artifacts, and observable data models. Internal implementation details are treated as private. If an implementation changes its internal data structures but preserves the external contract, tests must continue to pass without modification.

### 2.2 Anti-Facade & Genuine Assertion Integrity
Facade tests—tests that pass unconditionally, assert trivial tautologies (`assert!(true)`), mock away the critical path, or hardcode expected hashes without exercising the underlying algorithms—are strictly banned. Every test case in this suite:
- Executes the genuine algorithms (atomic tempfile writes, Sugiyama graph layouts, regex tokenizers, zip decompressors).
- Inspects real filesystem side-effects (file creation, directory tree integrity, file modification timestamps, durability after write).
- Verifies exact error variants when handling malicious or boundary inputs.

### 2.3 Hermetic & Self-Contained Execution
Each test case creates its own isolated sandbox environment using `tempfile::tempdir()`. Tests never share mutable state, do not rely on execution order, and automatically clean up all disk artifacts upon completion. No test attempts network communication or touches user-level application directories.

### 2.4 Deterministic Expected Value Derivation
Expected values in this test suite are derived directly from authoritative mathematical properties, standards specifications, and system contracts:
- **CommonMark / GFM:** Derived from GitHub Flavored Markdown Spec (GFM 0.29).
- **Mermaid:** Derived from Mermaid syntax definitions across all 7 supported diagram types.
- **Windows Device Names:** Derived from Microsoft Windows Win32 Namespace Specifications (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`).
- **Zip-Slip & Path Traversal:** Derived from OWASP Path Traversal & Zip-Slip Defense Standards.

---

## 3. Feature Inventory & Verification Matrix

The 77 features catalogued in `requirements.md` are mapped across the 4-tier E2E testing framework:

| Category | Feature Name | Description | Verification Tier | Primary Test Target |
|:---|:---|:---|:---|:---|
| **ARCH** | Multi-Crate Workspace | Decoupled `core`, `render`, `app` crates | Tier 1 | Workspace compile & module boundaries |
| **ARCH** | Non-Blocking UI Thread | Heavy I/O, search, rendering off UI thread | Tier 3, 4 | Concurrency & background workers |
| **VAULT** | Local 1:1 Directory Mapping | Real folder hierarchy on disk | Tier 1, 4 | `Vault::open`, tree scanning |
| **VAULT** | Note Creation | Create `Untitled.md` with auto-increment | Tier 1, 4 | `resolve_unique_note_name` |
| **VAULT** | Folder Creation | Subdirectory creation inside vault | Tier 1, 4 | `create_folder`, tree update |
| **VAULT** | In-Place Renaming | Safe renaming with conflict guard | Tier 1 | `rename_item`, collision validation |
| **VAULT** | Safe Deletion & Recycle Bin | OS recycle bin with `.trash/` fallback | Tier 1, 3 | `safe_delete_item`, `.trash/` timestamp |
| **SAFE** | Atomic Note Writes | Tempfile + fsync + rename protocol | Tier 1 | `atomic_write_file`, crash resilience |
| **SAFE** | Strict Path Sanitization | Vault containment, reject `..` traversal | Tier 1, 2 | `sanitize_relative_path` |
| **SAFE** | Windows Reserved Names | Block `CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9` | Tier 1 | Windows reserved name rejection |
| **SAFE** | Windows Illegal Chars | Block `<>:"/\|?*` & trailing dots/spaces | Tier 1, 2 | Filename sanitization |
| **SAFE** | Windows Long Paths | Extended length paths (>260 chars) | Tier 2 | Path nesting > 260 chars |
| **SAFE** | Line Ending Preservation | Preserve CRLF vs LF, default LF | Tier 2 | Line ending round-trip detection |
| **SAFE** | UTF-8 BOM Tolerance | Strip/handle `\u{FEFF}` seamlessly | Tier 2 | Zero artifact display on BOM notes |
| **SAFE** | Conflict Resolution | External disk changes vs dirty buffer | Tier 3 | 3-way conflict detection & state |
| **EDIT** | Large Note Scalability | 1 MB+ plain-text note handling | Tier 2 | 1 MB note parsing & memory budget |
| **PREV** | CommonMark + GFM | Headings, lists, bold, italic, code | Tier 1, 4 | `DocModel` parsing & formatting |
| **PREV** | Markdown Tables | GFM tables with column alignments | Tier 1, 4 | Table AST block parsing |
| **PREV** | Task Lists | GFM interactive `- [ ]` and `- [x]` | Tier 1 | Task item parsed checkboxes |
| **PREV** | Code Syntax Highlighting | Pure-Rust syntect code fence colors | Tier 1, 4 | Syntax highlighted spans |
| **MERM** | Native Mermaid Engine | Browserless Rust parsing & layout | Tier 1 | Pure SVG generation |
| **MERM** | Flowchart / Graph | `graph TD`, `flowchart LR`, subgraphs | Tier 1 | Flowchart SVG layout & nodes |
| **MERM** | Sequence Diagram | `sequenceDiagram`, actors, lifelines | Tier 1 | Sequence SVG participants & messages |
| **MERM** | Class Diagram | `classDiagram`, compartments, methods | Tier 1 | Class SVG nodes & associations |
| **MERM** | State Diagram | `stateDiagram-v2`, transitions, states | Tier 1 | State machine SVG nodes & arrows |
| **MERM** | ER Diagram | `erDiagram`, entities, crows-feet | Tier 1 | ER SVG entity boxes & relationships |
| **MERM** | Pie Chart | `pie title`, radial slices, labels | Tier 1 | Pie SVG `<path>` arcs & percentages |
| **MERM** | Gantt Chart | `gantt`, timeline, sections, tasks | Tier 1 | Gantt SVG timeline axes & task bars |
| **MERM** | Graceful Error Fallback | Formatted code box with diagnostic | Tier 2 | Zero panic on malformed Mermaid |
| **SRCH** | Fuzzy Quick Open | Filename fuzzy matching | Tier 1, 3 | `quick_open_search` scoring |
| **SRCH** | Global Full-Text Search | Multi-threaded parallel regex scan | Tier 3 | Streaming text search during writes |
| **IMP** | Safe Zip Archive Import | Zip-slip and zip-bomb safe extraction | Tier 1, 2 | Zip traversal & threshold rejection |
| **IMP** | Import Conflict Policies | Skip, Overwrite, Keep Both | Tier 3 | `KeepBoth` suffix naming |
| **EXP** | Vault Zip Backup | Full vault export to `.zip` archive | Tier 1, 4 | Archive round-trip inspection |
| **EXP** | Standalone HTML Export | Self-contained HTML with inline SVG | Tier 1, 4 | Exported HTML document structure |
| **PRIV** | Zero-Network Enforcement | Strict ban of HTTP client libraries | Tier 1 | Dependency tree inspection |

---

## 4. Four-Tier Testing Methodology

```
┌────────────────────────────────────────────────────────────────────────┐
│                   Tier 4: Real-World User Workflows                    │
│   (Vault initialization, Markdown authoring, HTML export, Zip backup)   │
├────────────────────────────────────────────────────────────────────────┤
│              Tier 3: Cross-Feature Interactions & Concurrency          │
│ (External disk edit race, Trash fallback, Import conflict, Live search)│
├────────────────────────────────────────────────────────────────────────┤
│                 Tier 2: Boundary & Corner Cases                        │
│   (0-byte notes, 1MB notes, >260 char paths, CRLF, BOM, Bad Mermaid)   │
├────────────────────────────────────────────────────────────────────────┤
│                   Tier 1: Core Feature Verification                    │
│  (Atomic writes, Path safety, Windows names, 7 Mermaid types, Export)  │
└────────────────────────────────────────────────────────────────────────┘
```

### 4.1 Tier 1: Core Feature Integration Harness (`tier1_feature_tests.rs`)
Tier 1 verifies that each foundational feature performs its isolated contract correctly under normal conditions:
1. **Atomic Note Writes:** Writes content through a hidden sibling temporary file (`.<name>.tmp.<timestamp>`), flushes buffers, calls `sync_all()`, and renames atomically over the target. Verifies data durability and absence of orphaned temp files.
2. **Strict Path Sanitization:** Rejects absolute paths (`/etc/passwd`, `C:\Windows`), directory traversals (`../outside.md`, `foo/../../bar.md`), and symlinks resolving outside the vault boundary.
3. **Windows Reserved Device Names:** Blocks reserved names regardless of casing or file extension (`CON`, `con.md`, `prn.txt`, `aux`, `nul.markdown`, `COM1` through `COM9`, `LPT1` through `LPT9`).
4. **Collision Suffix Naming:** Creates notes in sequence (`Untitled.md`, `Untitled 1.md`, `Untitled 2.md`), verifying that existing notes are never overwritten.
5. **CommonMark & GFM Parsing:** Parses headings (levels 1–6), bold, italic, strikethrough, blockquotes, horizontal rules, task lists (`- [x] Done`), and GFM tables with column alignment.
6. **Native Mermaid Diagram SVG Generation:** Tests all 7 required diagram types:
   - Flowchart / Graph (`graph TD`)
   - Sequence Diagram (`sequenceDiagram`)
   - Class Diagram (`classDiagram`)
   - State Diagram (`stateDiagram-v2`)
   - Entity-Relationship Diagram (`erDiagram`)
   - Pie Chart (`pie title`)
   - Gantt Chart (`gantt`)
   Verifies that valid, well-formed `<svg ...>...</svg>` markup is produced natively without browser or webview dependencies.
7. **Zip-Slip Path Traversal Rejection:** Inspects malicious zip archives containing paths with `../` or `..\` and ensures extraction is aborted before writing outside target directory.
8. **Export Subsystems:** Validates full vault export to a compressed `.zip` archive and single-note export to standalone HTML containing inline SVG graphics.

### 4.2 Tier 2: Boundary & Resource Harness (`tier2_boundary_tests.rs`)
Tier 2 stress-tests boundary limits, resource exhaustion guards, and platform-specific quirks:
1. **Zero-Byte Empty Notes:** Creates, reads, renders, and exports 0-byte `.md` files without errors, division-by-zero, or rendering crashes.
2. **Large Note Scalability (1 MB+):** Synthesizes a massive Markdown document exceeding 1 Megabyte (20,000+ lines with headings, lists, code blocks, and tables). Verifies that parser produces a valid document model in <500 ms without memory spikes.
3. **Deeply Nested Paths (>260 Characters):** Constructs a directory structure exceeding standard Windows `MAX_PATH` (260 characters). Tests path sanitization, creation, and atomic writes on extended paths.
4. **CRLF vs LF Line Ending Preservation:**
   - Detects CRLF (`\r\n`) in existing Windows notes and preserves `\r\n` upon editing and saving.
   - Detects LF (`\n`) in Unix notes and preserves `\n`.
   - Defaults newly created notes to LF (`\n`).
5. **UTF-8 Byte Order Mark (BOM) Tolerance:** Reads files beginning with `\u{FEFF}` (UTF-8 BOM), verifying that the text is processed cleanly without displaying corrupted `ï»¿` artifacts in the editor or preview.
6. **Malformed Mermaid Error Diagnostics:** Feeds corrupted, unclosed, or invalid Mermaid syntax to the parser. Verifies that the engine **never panics**, captures errors gracefully, and returns structured diagnostics with line numbers.
7. **Zip Bomb Threshold Guard:** Constructs an archive containing excessive entries or compression ratios exceeding safety thresholds (>10,000 files or >50 MB single-file uncompressed). Verifies that the extraction routine rejects the archive before exhausting host memory or disk space.

### 4.3 Tier 3: Cross-Feature Interactions Harness (`tier3_cross_feature_tests.rs`)
Tier 3 tests complex interactions between multiple concurrent subsystems:
1. **External Disk Modification During Active Edits:**
   - Simulates an open note buffer with unsaved modifications in memory while an external process (e.g. Git or an external editor) overwrites the file on disk.
   - Verifies conflict detection: the application detects the external change, avoids silent overwrite, and isolates the in-memory buffer until conflict resolution ("Keep Mine" / "Take Theirs" / "View Diff").
2. **Safe Deletion with Trash Fallback:**
   - Tests file deletion routing to OS trash.
   - If OS trash is unavailable or returns an error (e.g. in headless CI or certain container filesystems), automatically falls back to moving the file into `<vault>/.trash/<timestamp>_<filename>`.
3. **Import Conflict Policy ("Keep Both"):**
   - Imports a folder containing files with names that collide with existing vault files under the `KeepBoth` policy.
   - Verifies that existing files remain intact and imported files are safely renamed with collision suffixes (e.g. `Readme (1).md`).
4. **Search Concurrency During File Modifications:**
   - Executes background multi-threaded regex searches across the vault while files are actively created, modified, and renamed on disk.
   - Verifies that file locks or transient file states do not crash the search engine or cause read deadlocks.

### 4.4 Tier 4: Real-World User Scenarios (`tier4_real_world_tests.rs`)
Tier 4 simulates complete end-to-end user journeys from beginning to end:
- **Scenario A (Vault Initialization):** User initializes a new vault, configures settings, creates a multi-level folder hierarchy (`Projects/AirGap/Docs`), and populates notes.
- **Scenario B (Rich Markdown Authoring):** User authors a comprehensive technical specification containing GFM tables, task checklists, syntax-highlighted code fences, and embedded Mermaid architecture diagrams.
- **Scenario C (Standalone HTML Export):** User exports the technical specification to a self-contained HTML file. The test inspects the HTML output to verify that CSS is embedded, code blocks are highlighted, and Mermaid diagrams are included as clean inline `<svg>` elements.
- **Scenario D (Vault Backup & Disaster Recovery):** User creates a full `.zip` backup of the vault. The test deletes the original vault from disk, restores it completely from the `.zip` archive, and asserts that 100% of files, directory structures, and file contents match byte-for-byte.

---

## 5. Test Runner Instructions

All tests are implemented using standard Rust `cargo test` integration targets and are configured to execute in any environment (Windows, Linux, macOS, headless CI).

### 5.1 Running the Complete E2E Test Suite
To run all 4 tiers of E2E integration tests:

```bash
# Standalone test runner (from workspace root):
cargo test --manifest-path tests/Cargo.toml

# Or running all 4 tiers explicitly:
cargo test --manifest-path tests/Cargo.toml --test tier1_feature_tests --test tier2_boundary_tests --test tier3_cross_feature_tests --test tier4_real_world_tests

# From workspace root (when test package is registered in workspace members):
cargo test -p airgap-tests
```

### 5.2 Running Individual Tiers
You can run any specific test tier independently:

```bash
# Tier 1: Core Features
cargo test --manifest-path tests/Cargo.toml --test tier1_feature_tests -- --nocapture

# Tier 2: Boundary & Corner Cases
cargo test --manifest-path tests/Cargo.toml --test tier2_boundary_tests -- --nocapture

# Tier 3: Cross-Feature Interactions
cargo test --manifest-path tests/Cargo.toml --test tier3_cross_feature_tests -- --nocapture

# Tier 4: Real-World End-to-End Workflows
cargo test --manifest-path tests/Cargo.toml --test tier4_real_world_tests -- --nocapture
```

### 5.3 Filtering Specific Test Cases
Filter by test name:

```bash
# Test Mermaid diagram rendering specifically
cargo test --test tier1_feature_tests test_mermaid -- --nocapture

# Test Windows reserved names sanitization
cargo test --test tier1_feature_tests test_windows_reserved -- --nocapture

# Test 1 MB note scalability
cargo test --test tier2_boundary_tests test_large_note -- --nocapture

# Test conflict resolution
cargo test --test tier3_cross_feature_tests test_external_disk_change -- --nocapture
```

### 5.4 Zero-Network Dependency Audit
To assert that no HTTP client libraries (`reqwest`, `hyper`, `curl`, `ureq`) exist in the dependency tree:

```bash
cargo tree --target all | findstr /i "reqwest hyper curl ureq"
```
*(On Linux: `cargo tree --target all | grep -Ei "reqwest|hyper|curl|ureq"`)*  
The command must produce zero matches.

---

## 6. Defect Escalation & Reporting Protocol

If an E2E test fails:
1. **Determine Failure Classification:**
   - **Contract Violation:** Implementation does not conform to `PROJECT.md` or `AGENT.md`.
   - **Security / Safety Failure:** Path traversal, Windows reserved name bypass, or atomic write loss detected.
   - **Performance / Memory Violation:** 1 MB note took > 500 ms to parse, or Mermaid rendering exceeded memory budget.
2. **Escalate to Implementing Agent:**
   - Capture exact reproduction steps, input parameters, observed error message, and expected behavior.
   - File handoff report with verbatim failure logs.
   - **Do not modify implementation code in `crates/**`** (strictly adhering to test writer separation of duties).
