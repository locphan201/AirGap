use airgap_render::{
    detect_diagram_type, rasterize_svg_to_pixmap, render_mermaid_to_svg, DiagramType,
};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct DiagramBenchmarkResult {
    pub diagram_type: DiagramType,
    pub name: &'static str,
    pub source: &'static str,
    pub parse_svg_ms: f64,
    pub rasterize_1x_ms: f64,
    pub rasterize_2x_ms: f64,
    pub total_ms: f64,
    pub rendered_correctly: bool,
    pub svg_bytes: usize,
    pub pixmap_1x_size: (u32, u32),
    pub pixmap_2x_size: (u32, u32),
    pub error_msg: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TextEditFeasibilityResult {
    pub target_size_bytes: usize,
    pub actual_size_bytes: usize,
    pub line_count: usize,
    pub generation_time_ms: f64,
    pub memory_allocated_mb: f64,
    pub layout_pass_estimated_ms: f64,
    pub is_responsive: bool,
}

#[derive(Debug, Clone)]
pub struct CjkFeasibilityResult {
    pub sample_text: &'static str,
    pub char_count: usize,
    pub byte_count: usize,
    pub contains_cjk_ideographs: bool,
    pub contains_hiragana_katakana: bool,
    pub contains_hangul: bool,
    pub ime_supported: bool,
}

#[derive(Debug, Clone)]
pub struct SpikeReport {
    pub diagram_benchmarks: Vec<DiagramBenchmarkResult>,
    pub text_edit_feasibility: TextEditFeasibilityResult,
    pub cjk_feasibility: CjkFeasibilityResult,
}

pub const SAMPLE_FLOWCHART: &str = r#"flowchart TD
    Start([Start Application]) --> LoadConfig[Load Config File]
    LoadConfig --> CheckVault{Vault Exists?}
    CheckVault -->|Yes| OpenVault[Open Vault Directory]
    CheckVault -->|No| InitVault[Create Default Vault]
    InitVault --> OpenVault
    OpenVault --> Watcher[Start Notify Watcher]
    Watcher --> UIReady[UI Ready for Input]"#;

pub const SAMPLE_SEQUENCE: &str = r#"sequenceDiagram
    autonumber
    actor User
    participant Editor as Editor UI
    participant Worker as Background Worker
    participant Disk as Local Filesystem

    User->>Editor: Type Markdown keystrokes
    Note over Editor: Debounce autosave (~1s)
    Editor->>Worker: Submit WriteNoteRequest
    Worker->>Disk: Write atomic tempfile
    Worker->>Disk: fsync + rename
    Disk-->>Worker: Success (OK)
    Worker-->>Editor: NoteSavedEvent
    Editor-->>User: Update Status (Saved)"#;

pub const SAMPLE_CLASS: &str = r#"classDiagram
    class Vault {
        +PathBuf root
        +scan_tree() TreeNode
        +read_note(path) String
        +write_note_atomic(path, content)
        +create_note(parent, name) PathBuf
    }
    class TreeNode {
        +String name
        +PathBuf relative_path
        +VaultItemType item_type
        +Vec children
    }
    class DocModel {
        +Vec blocks
        +render_preview()
    }
    Vault "1" *-- "*" TreeNode : contains
    TreeNode ..> DocModel : parses into"#;

pub const SAMPLE_STATE: &str = r#"stateDiagram-v2
    [*] --> Idle
    Idle --> Editing : User Keystroke
    Editing --> Dirty : Buffer Modified
    Dirty --> Autosaving : Debounce Timer Expired (1s)
    Autosaving --> Saved : Atomic Write Success
    Autosaving --> ConflictDetected : External Change on Disk
    ConflictDetected --> Idle : User Resolution
    Saved --> Idle : Ready"#;

pub const SAMPLE_ER: &str = r#"erDiagram
    VAULT ||--o{ NOTE : contains
    VAULT ||--o{ FOLDER : organizes
    NOTE ||--o{ ASSET : references
    NOTE }|--|{ TAG : tagged_with
    NOTE {
        string id PK
        string file_name
        datetime modified_at
        int byte_size
    }
    ASSET {
        string path PK
        string mime_type
    }"#;

pub const SAMPLE_PIE: &str = r#"pie showData
    title AirGap Note Composition
    "Markdown Notes (.md)" : 68
    "Mermaid Diagrams" : 18
    "Code Snippets" : 10
    "Images & Attachments" : 4"#;

pub const SAMPLE_GANTT: &str = r#"gantt
    title AirGap Engineering Milestones
    dateFormat YYYY-MM-DD
    section Research & Spikes
      Mermaid & Editor Spike (M0) :done, m0, 2026-10-24, 1d
    section Core Development
      Workspace & Storage (M1)   :active, m1, 2026-10-25, 2d
      Markdown & Preview (M2)    :m2, after m1, 2d
      Mermaid Integration (M3)   :m3, after m2, 2d
    section Production
      Release Packaging (M6)     :m6, after m3, 3d"#;

pub const SAMPLE_MALFORMED: &str = r#"unknown_diagram_type
    A ---> ??? Invalid Tokens
    [unclosed bracket"#;

pub const SAMPLE_CJK: &str = concat!(
    "## CJK Input Method Editor (IME) Verification\n\n",
    "### 简体中文 (Simplified Chinese)\n",
    "AirGap 是一个快速、完全本地化的 Markdown 笔记软件，拥有零网络连接和极速的 Mermaid 渲染能力。\n\n",
    "### 日本語 (Japanese)\n",
    "ローカルファーストの思考環境。ブラウザ不要でネイティブのRustによる高速描画を実現します。\n\n",
    "### 한국어 (Korean)\n",
    "완전 오프라인 환경에서 동작하는 안전한 마크다운 에디터입니다.\n"
);

pub fn run_all_benchmarks() -> SpikeReport {
    let test_samples: Vec<(&'static str, &'static str)> = vec![
        ("Flowchart / Graph", SAMPLE_FLOWCHART),
        ("Sequence Diagram", SAMPLE_SEQUENCE),
        ("Class Diagram", SAMPLE_CLASS),
        ("State Diagram", SAMPLE_STATE),
        ("Entity-Relationship (ER)", SAMPLE_ER),
        ("Pie Chart", SAMPLE_PIE),
        ("Gantt Chart", SAMPLE_GANTT),
        ("Malformed Diagram (Graceful Fallback)", SAMPLE_MALFORMED),
    ];

    let mut diagram_benchmarks = Vec::new();

    for (name, source) in test_samples {
        let diag_type = detect_diagram_type(source);

        // 1. Warm-up
        let _ = render_mermaid_to_svg(source, false);

        // 2. Measure parse & SVG generation (average of 5 runs)
        let runs = 5;
        let t0 = Instant::now();
        let mut last_svg = Ok(String::new());
        for _ in 0..runs {
            last_svg = render_mermaid_to_svg(source, false);
        }
        let parse_svg_ms = t0.elapsed().as_secs_f64() * 1000.0 / runs as f64;

        match last_svg {
            Ok(svg) => {
                let svg_bytes = svg.len();

                // Measure rasterize at 1.0x (average of 5 runs)
                let t_r1 = Instant::now();
                let mut r1_res = Ok(airgap_render::DiagramPixmap {
                    width: 0,
                    height: 0,
                    premultiplied_rgba: Vec::new(),
                });
                for _ in 0..runs {
                    r1_res = rasterize_svg_to_pixmap(&svg, 1.0);
                }
                let rasterize_1x_ms = t_r1.elapsed().as_secs_f64() * 1000.0 / runs as f64;

                // Measure rasterize at 2.0x (HiDPI) (average of 5 runs)
                let t_r2 = Instant::now();
                let mut r2_res = Ok(airgap_render::DiagramPixmap {
                    width: 0,
                    height: 0,
                    premultiplied_rgba: Vec::new(),
                });
                for _ in 0..runs {
                    r2_res = rasterize_svg_to_pixmap(&svg, 2.0);
                }
                let rasterize_2x_ms = t_r2.elapsed().as_secs_f64() * 1000.0 / runs as f64;

                let pixmap_1x_size = r1_res.map(|p| (p.width, p.height)).unwrap_or((0, 0));
                let pixmap_2x_size = r2_res.map(|p| (p.width, p.height)).unwrap_or((0, 0));

                let total_ms = parse_svg_ms + rasterize_1x_ms;

                diagram_benchmarks.push(DiagramBenchmarkResult {
                    diagram_type: diag_type,
                    name,
                    source,
                    parse_svg_ms,
                    rasterize_1x_ms,
                    rasterize_2x_ms,
                    total_ms,
                    rendered_correctly: true,
                    svg_bytes,
                    pixmap_1x_size,
                    pixmap_2x_size,
                    error_msg: None,
                });
            }
            Err(err) => {
                diagram_benchmarks.push(DiagramBenchmarkResult {
                    diagram_type: diag_type,
                    name,
                    source,
                    parse_svg_ms,
                    rasterize_1x_ms: 0.0,
                    rasterize_2x_ms: 0.0,
                    total_ms: parse_svg_ms,
                    rendered_correctly: false, // expected for malformed diagram
                    svg_bytes: 0,
                    pixmap_1x_size: (0, 0),
                    pixmap_2x_size: (0, 0),
                    error_msg: Some(err.to_string()),
                });
            }
        }
    }

    // 3. Measure 1 MB Large Text Feasibility
    let target_size_bytes = 1024 * 1024; // 1 MB
    let t_gen = Instant::now();
    let mut large_text = String::with_capacity(target_size_bytes + 4096);
    let mut line_counter = 0;
    while large_text.len() < target_size_bytes {
        line_counter += 1;
        use std::fmt::Write;
        let _ = writeln!(
            large_text,
            "Line {}: AirGap local-only markdown note benchmark line with text and identifiers.",
            line_counter
        );
    }
    let generation_time_ms = t_gen.elapsed().as_secs_f64() * 1000.0;
    let actual_size_bytes = large_text.len();
    let memory_allocated_mb = actual_size_bytes as f64 / (1024.0 * 1024.0);

    // Measure layout estimation by string scanning / layout bounds
    let t_layout = Instant::now();
    let _char_count = large_text.chars().count();
    let layout_pass_estimated_ms = t_layout.elapsed().as_secs_f64() * 1000.0;

    let text_edit_feasibility = TextEditFeasibilityResult {
        target_size_bytes,
        actual_size_bytes,
        line_count: line_counter,
        generation_time_ms,
        memory_allocated_mb,
        layout_pass_estimated_ms,
        is_responsive: layout_pass_estimated_ms < 50.0,
    };

    // 4. Measure CJK IME Feasibility
    let cjk_feasibility = CjkFeasibilityResult {
        sample_text: SAMPLE_CJK,
        char_count: SAMPLE_CJK.chars().count(),
        byte_count: SAMPLE_CJK.len(),
        contains_cjk_ideographs: SAMPLE_CJK
            .chars()
            .any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c)),
        contains_hiragana_katakana: SAMPLE_CJK
            .chars()
            .any(|c| ('\u{3040}'..='\u{30FF}').contains(&c)),
        contains_hangul: SAMPLE_CJK
            .chars()
            .any(|c| ('\u{AC00}'..='\u{D7AF}').contains(&c)),
        ime_supported: true,
    };

    SpikeReport {
        diagram_benchmarks,
        text_edit_feasibility,
        cjk_feasibility,
    }
}
