use airgap_render::{rasterize_svg_to_pixmap, render_mermaid_to_svg};
use std::time::Instant;

const SAMPLE_FLOWCHART: &str = r#"flowchart TD
    Start([Start Application]) --> LoadConfig[Load Config File]
    LoadConfig --> CheckVault{Vault Exists?}
    CheckVault -->|Yes| OpenVault[Open Vault Directory]
    CheckVault -->|No| InitVault[Create Default Vault]
    InitVault --> OpenVault
    OpenVault --> Watcher[Start Notify Watcher]
    Watcher --> UIReady[UI Ready for Input]"#;

const SAMPLE_SEQUENCE: &str = r#"sequenceDiagram
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

const SAMPLE_CLASS: &str = r#"classDiagram
    class Vault {
        +PathBuf root
        +scan_tree() TreeNode
        +read_note(path) String
        +write_note_atomic(path, content)
    }
    class TreeNode {
        +String name
        +PathBuf relative_path
    }
    Vault "1" *-- "*" TreeNode : contains"#;

const SAMPLE_STATE: &str = r#"stateDiagram-v2
    [*] --> Idle
    Idle --> Editing : Keystroke
    Editing --> Dirty : Buffer Modified
    Dirty --> Autosaving : Debounce Timer (1s)
    Autosaving --> Saved : Atomic Write
    Saved --> Idle : Ready"#;

const SAMPLE_ER: &str = r#"erDiagram
    VAULT ||--o{ NOTE : contains
    VAULT ||--o{ FOLDER : organizes
    NOTE ||--o{ ASSET : references
    NOTE {
        string id PK
        string file_name
    }"#;

const SAMPLE_PIE: &str = r#"pie showData
    title AirGap Composition
    "Notes" : 70
    "Diagrams" : 20
    "Assets" : 10"#;

const SAMPLE_GANTT: &str = r#"gantt
    title Engineering Schedule
    dateFormat YYYY-MM-DD
    section Core
      Storage (M1) :done, 2026-10-24, 2d
      Renderer (M2) :active, 2026-10-26, 2d"#;

#[test]
fn test_hidpi_rasterization_scales_and_premultiplied_alpha_invariants() {
    let samples: Vec<(&'static str, &'static str)> = vec![
        ("Flowchart", SAMPLE_FLOWCHART),
        ("Sequence", SAMPLE_SEQUENCE),
        ("Class", SAMPLE_CLASS),
        ("State", SAMPLE_STATE),
        ("ER", SAMPLE_ER),
        ("Pie", SAMPLE_PIE),
        ("Gantt", SAMPLE_GANTT),
    ];

    let scales = [1.0f32, 1.5f32, 2.0f32, 3.0f32];

    for (name, source) in samples {
        for &is_dark in &[false, true] {
            let theme_label = if is_dark { "Dark" } else { "Light" };
            let svg = render_mermaid_to_svg(source, is_dark).unwrap_or_else(|e| {
                panic!(
                    "Failed to generate SVG for {} ({}): {:?}",
                    name, theme_label, e
                )
            });

            let mut base_dims = (0u32, 0u32);

            for &scale in &scales {
                let t0 = Instant::now();
                let pixmap = rasterize_svg_to_pixmap(&svg, scale).unwrap_or_else(|e| {
                    panic!(
                        "Rasterization failed for {} at {:.1}x: {:?}",
                        name, scale, e
                    )
                });
                let raster_ms = t0.elapsed().as_secs_f64() * 1000.0;

                assert_eq!(
                    pixmap.premultiplied_rgba.len(),
                    (pixmap.width * pixmap.height * 4) as usize,
                    "Pixmap byte buffer length must match width * height * 4"
                );

                if (scale - 1.0).abs() < 0.001 {
                    base_dims = (pixmap.width, pixmap.height);
                } else {
                    // Check scale proportionality (within 2px ceiling delta)
                    let expected_w = (base_dims.0 as f32 * scale).ceil() as u32;
                    let expected_h = (base_dims.1 as f32 * scale).ceil() as u32;
                    assert!(
                        (pixmap.width as i32 - expected_w as i32).abs() <= 2,
                        "Width scaling deviation at {:.1}x: got {}, expected {}",
                        scale,
                        pixmap.width,
                        expected_w
                    );
                    assert!(
                        (pixmap.height as i32 - expected_h as i32).abs() <= 2,
                        "Height scaling deviation at {:.1}x: got {}, expected {}",
                        scale,
                        pixmap.height,
                        expected_h
                    );
                }

                // Verify Premultiplied Alpha Mathematical Invariants
                let mut semi_transparent_pixels = 0usize;
                let mut fully_transparent_pixels = 0usize;
                let mut fully_opaque_pixels = 0usize;
                let mut invalid_premultiplied_pixels = 0usize;

                for chunk in pixmap.premultiplied_rgba.as_chunks::<4>().0 {
                    let r = chunk[0];
                    let g = chunk[1];
                    let b = chunk[2];
                    let a = chunk[3];

                    // Premultiplied alpha invariant: r <= a, g <= a, b <= a
                    if r > a || g > a || b > a {
                        invalid_premultiplied_pixels += 1;
                    }

                    if a == 0 {
                        fully_transparent_pixels += 1;
                        // Zero alpha must have zero color channels
                        assert!(
                            r == 0 && g == 0 && b == 0,
                            "Transparent pixel must have r=g=b=0, found ({}, {}, {}, {})",
                            r,
                            g,
                            b,
                            a
                        );
                    } else if a == 255 {
                        fully_opaque_pixels += 1;
                    } else {
                        semi_transparent_pixels += 1;
                    }
                }

                println!(
                    "[{:10} | {:5} | {:.1}x] Size: {:4}x{:4} | {:.2} ms | Opaque: {:7} | Semi: {:6} | Trans: {:7}",
                    name,
                    theme_label,
                    scale,
                    pixmap.width,
                    pixmap.height,
                    raster_ms,
                    fully_opaque_pixels,
                    semi_transparent_pixels,
                    fully_transparent_pixels
                );

                assert_eq!(
                    invalid_premultiplied_pixels, 0,
                    "Found {} invalid premultiplied pixels in {} at scale {:.1}x",
                    invalid_premultiplied_pixels, name, scale
                );

                // Note: Dark theme has an opaque background rectangle in mermaid-rs-renderer Theme::dark(),
                // and Pie chart has a full-frame solid background rect in light theme.
                // Light theme diagrams with transparent/fractional background produce semi-transparent edge pixels.
                if !is_dark && name != "Pie" {
                    assert!(
                        semi_transparent_pixels > 0,
                        "Anti-aliased vector rendering must produce semi-transparent edge pixels in {}",
                        name
                    );
                }

                if semi_transparent_pixels > 0 {
                    // For all semi-transparent pixels, verify that r, g, b are strictly within alpha bounds
                    for chunk in pixmap.premultiplied_rgba.as_chunks::<4>().0 {
                        let r = chunk[0];
                        let g = chunk[1];
                        let b = chunk[2];
                        let a = chunk[3];
                        if a > 0 && a < 255 {
                            assert!(
                                r <= a && g <= a && b <= a,
                                "Premultiplied invariant violated"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test_dark_halo_prevention_simulation() {
    // Simulate blending semi-transparent edge pixels over a light background (255, 255, 255)
    // and demonstrate difference between premultiplied blending vs double-multiplication
    let svg = render_mermaid_to_svg(SAMPLE_FLOWCHART, false).unwrap();
    let pixmap = rasterize_svg_to_pixmap(&svg, 2.0).unwrap();

    let mut halo_delta_max = 0.0f32;
    let mut semi_pixels = 0;

    for chunk in pixmap.premultiplied_rgba.as_chunks::<4>().0 {
        let r_p = chunk[0] as f32;
        let a = chunk[3] as f32;

        if a > 0.0 && a < 255.0 {
            semi_pixels += 1;
            let alpha_norm = a / 255.0;

            // Correct Premultiplied Blend over White: C_p + C_bg * (1 - alpha)
            let correct_r = r_p + 255.0 * (1.0 - alpha_norm);

            // Buggy Double-Multiplied Blend (Dark Halo artifact): (C_p * alpha) + C_bg * (1 - alpha)
            let halo_r = (r_p * alpha_norm) + 255.0 * (1.0 - alpha_norm);

            let delta = (correct_r - halo_r).abs();
            if delta > halo_delta_max {
                halo_delta_max = delta;
            }
        }
    }

    println!(
        "[Dark Halo Analysis] Analyzed {} edge pixels: Maximum halo darkening prevented = {:.1} color levels (out of 255)",
        semi_pixels, halo_delta_max
    );
    assert!(
        halo_delta_max > 10.0,
        "Dark halo artifact would cause significant darkening (>10 levels) if not premultiplied"
    );
}
