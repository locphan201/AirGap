## 2026-10-25T00:16:06Z
From: 3afb76bf-62b1-4f73-84eb-b7f21076aae2 (Parent Orchestrator)
Content:
You are explorer_m2_3 (Role: Preview UI Explorer).
Your working directory is: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_3
Workspace root: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap

Mandatory files to read:
- ORIGINAL_REQUEST.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\ORIGINAL_REQUEST.md
- AGENT.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\AGENT.md
- PROJECT.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap\PROJECT.md
- TEST_INFRA.md: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap\TEST_INFRA.md
- Existing crates: `airgap/crates/core`, `airgap/crates/render`, `airgap/crates/app`

Mission for Milestone M2:
Investigate and design the Desktop GUI Preview & Editor enhancements in `airgap/crates/app`:
1. Triple View Modes: Editor Only, Split View (50/50 resizable split), Preview Only (`Ctrl+\` cycle accelerator or toolbar button).
2. Approximate Editor-Preview scroll synchronization: how egui `ScrollArea` state maps between cursor/line position and rendered preview elements.
3. Safe external hyperlink handling: clicking `http(s)://` or `mailto:` opens default OS browser via the `open` crate (pure local launch, zero HTTP client network stack in app).
4. Editor interactive features:
   - Line numbers gutter and active line highlight
   - Undo/redo stack verification
   - In-note Find & Replace bar (`Ctrl+F` / `Ctrl+H` with match highlight, case/whole word/regex options)
   - Auto-pairing of brackets `()`, `[]`, `{}`, quotes `""`, backticks ````
   - List continuation on Enter (`- `, `* `, `1. `, `- [ ] `)
   - Indent / outdent with Tab / Shift+Tab
   - Smart URL paste: wrap selected text into `[selected](pasted_url)`
   - Image paste: clipboard image -> save to `assets/` -> insert `![image](assets/...)`
5. Outline recommended types, signatures, and implementation plan for the worker.

Write your complete analysis and recommendations to `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_3\handoff.md`.
Send a completion message back to your caller when done.
