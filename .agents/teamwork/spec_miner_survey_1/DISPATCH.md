## 2026-10-24T14:57:39Z
Sender: 85535336-9f0c-4c83-bb3f-dac37c864962
Priority: MESSAGE_PRIORITY_HIGH

You are spec_miner_survey_1.
Your working directory: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\spec_miner_survey_1
Authoritative User Request: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\ORIGINAL_REQUEST.md
Product Specification & Architecture: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\AGENT.md

MANDATORY: You MUST read ORIGINAL_REQUEST.md and AGENT.md before starting work.
Your task is to conduct a thorough specification extraction and feature inventory mining for the "AirGap" native Markdown note-taking desktop application in Rust.

Specifically:
1. Enumerate every functional requirement (R1-R6 in ORIGINAL_REQUEST.md and Sections 1-6 in AGENT.md):
   - Multi-crate workspace architecture (core, render, app)
   - Vault management, disk 1:1 mapping, atomic writes, path safety, conflict handling, watcher, deletion/trash
   - Markdown editor (syntax highlighting, line numbers, word wrap, autosave, conveniences, CJK/IME support)
   - Markdown preview (CommonMark+GFM, tables, code blocks, scroll sync, local images, links opening in OS browser only)
   - Native Mermaid diagram rendering (types: flowchart, sequence, class, state, ER, pie, gantt; rasterization via resvg/tiny-skia, caching, dark/light theming, HiDPI, graceful error fallback)
   - Search & navigation (quick open fuzzy match, global full-text search off UI thread)
   - Import/export (folder, Obsidian vault, zip with zip-slip/zip-bomb guards, HTML with inline SVG diagrams, backup export)
   - Non-negotiable privacy & local-only constraints (zero network calls, zero telemetry, zero HTTP clients in tree)
2. Enumerate every non-functional requirement, acceptance criterion, and edge case (e.g., Windows reserved names CON/PRN/AUX/NUL/COM1-9/LPT1-9, long paths, CRLF vs LF, 1MB note performance, <150MB RAM idle).
3. Structure your findings into a clear, categorized table of features with IDs, descriptions, priority, and verification criteria.
4. Write your complete report to C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\spec_miner_survey_1\requirements.md and provide a summary handoff in C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\spec_miner_survey_1\handoff.md. Update your progress.md regularly.
When finished, send a message back with the status and path to your handoff.
