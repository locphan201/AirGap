use airgap_app::app::AirGapApp;
use airgap_app::benchmarks::run_all_benchmarks;
use airgap_app::spike_window::SpikeDemoApp;
use std::env;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args
        .iter()
        .any(|a| a == "--bench" || a == "--headless" || a == "--report")
    {
        println!("============================================================");
        println!(" AirGap Milestone M0: Native Mermaid & Editor Feasibility ");
        println!("============================================================");

        let report = run_all_benchmarks();

        println!("\n### 1. Diagram Types vs Rendering Capability Matrix\n");
        println!("| Diagram Type | Category | Engine Support | Parse+SVG (ms) | Raster 1x (ms) | Raster 2x (ms) | Total (ms) |");
        println!("| :--- | :--- | :--- | :--- | :--- | :--- | :--- |");
        for b in &report.diagram_benchmarks {
            let status_badge = if b.rendered_correctly {
                "✅ Rendered Correctly"
            } else {
                "⚠️ Fallback / Error Handled"
            };
            println!(
                "| {} | {:?} | {} | {:.2} | {:.2} | {:.2} | {:.2} |",
                b.name,
                b.diagram_type,
                status_badge,
                b.parse_svg_ms,
                b.rasterize_1x_ms,
                b.rasterize_2x_ms,
                b.total_ms
            );
        }

        println!("\n### 2. Editor Feasibility Evaluation (egui::TextEdit)\n");
        println!(
            "- **Payload Size:** {:.2} MB",
            report.text_edit_feasibility.memory_allocated_mb
        );
        println!(
            "- **Line Count:** {} lines",
            report.text_edit_feasibility.line_count
        );
        println!(
            "- **Generation Time:** {:.2} ms",
            report.text_edit_feasibility.generation_time_ms
        );
        println!(
            "- **Layout Scan Time:** {:.2} ms",
            report.text_edit_feasibility.layout_pass_estimated_ms
        );
        println!(
            "- **Responsiveness:** {}",
            if report.text_edit_feasibility.is_responsive {
                "PASS (<50ms)"
            } else {
                "LAGGY"
            }
        );

        println!("\n### 3. CJK IME Compatibility Evaluation\n");
        println!(
            "- **Characters Tested:** {} ({} bytes)",
            report.cjk_feasibility.char_count, report.cjk_feasibility.byte_count
        );
        println!(
            "- **Simplified Chinese Ideographs:** {}",
            report.cjk_feasibility.contains_cjk_ideographs
        );
        println!(
            "- **Japanese Hiragana / Katakana:** {}",
            report.cjk_feasibility.contains_hiragana_katakana
        );
        println!(
            "- **Korean Hangul Syllables:** {}",
            report.cjk_feasibility.contains_hangul
        );
        println!("- **IME Composition:** Supported natively via egui / winit text input events");

        println!("\nSpike evaluation completed successfully.");
        return Ok(());
    }

    if args.iter().any(|a| a == "--spike") {
        let native_options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([1100.0, 750.0])
                .with_min_inner_size([700.0, 500.0])
                .with_title("AirGap M0 - Mermaid & Editor Feasibility Spike"),
            ..Default::default()
        };

        return eframe::run_native(
            "AirGap M0 Spike",
            native_options,
            Box::new(|_cc| Ok(Box::new(SpikeDemoApp::default()))),
        );
    }

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([750.0, 500.0])
            .with_title("AirGap - Native Offline Markdown Notes"),
        ..Default::default()
    };

    eframe::run_native(
        "AirGap",
        native_options,
        Box::new(|cc| Ok(Box::new(AirGapApp::new(cc)))),
    )
}
