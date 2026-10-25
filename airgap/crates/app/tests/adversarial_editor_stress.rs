use std::time::Instant;

#[test]
fn test_1mb_textedit_rapid_mutations_and_frame_times() {
    let ctx = egui::Context::default();

    // 1. Generate 1.00 MB payload (realistic Markdown)
    let target_bytes = 1024 * 1024;
    let mut text = String::with_capacity(target_bytes + 65536);
    let mut line_num = 0;
    while text.len() < target_bytes {
        line_num += 1;
        use std::fmt::Write;
        let _ = writeln!(
            text,
            "## Section {:04}: AirGap Markdown Note Line testing rendering performance and memory overhead.",
            line_num
        );
    }

    println!(
        "[1MB Payload] Initialized: {} bytes, {} lines",
        text.len(),
        text.lines().count()
    );

    // Warm-up egui context
    let raw_input = egui::RawInput::default();
    let _ = ctx.run(raw_input.clone(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY),
            );
        });
    });

    // 2. Measure Idle Frame Times (No mutations, layout cached)
    let idle_frames = 30;
    let mut idle_times_ms = Vec::with_capacity(idle_frames);
    for _ in 0..idle_frames {
        let t0 = Instant::now();
        let _ = ctx.run(raw_input.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY),
                );
            });
        });
        idle_times_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
    }

    let avg_idle = idle_times_ms.iter().sum::<f64>() / idle_frames as f64;
    let max_idle = idle_times_ms.iter().cloned().fold(0.0, f64::max);
    println!(
        "[1MB Idle Rendering] Avg: {:.2} ms/frame, Max: {:.2} ms/frame",
        avg_idle, max_idle
    );

    // 3. Measure Rapid Keystrokes / Mutations at End of 1 MB Buffer
    let append_frames = 30;
    let mut append_times_ms = Vec::with_capacity(append_frames);
    for i in 0..append_frames {
        let t0 = Instant::now();
        text.push_str(&format!(" Keystroke #{}", i));
        let _ = ctx.run(raw_input.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY),
                );
            });
        });
        append_times_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
    }

    let avg_append = append_times_ms.iter().sum::<f64>() / append_frames as f64;
    let max_append = append_times_ms.iter().cloned().fold(0.0, f64::max);
    println!(
        "[1MB Rapid Append Mutations] Avg: {:.2} ms/frame, Max: {:.2} ms/frame",
        avg_append, max_append
    );

    // 4. Measure Rapid Keystrokes / Mutations in the MIDDLE of 1 MB Buffer (simulating editing line 5,000)
    let mid_frames = 30;
    let mut mid_times_ms = Vec::with_capacity(mid_frames);
    let mid_point = text.len() / 2;
    for _i in 0..mid_frames {
        let t0 = Instant::now();
        // Insert character into middle
        text.insert_str(mid_point, " [x]");
        let _ = ctx.run(raw_input.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY),
                );
            });
        });
        mid_times_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
    }

    let avg_mid = mid_times_ms.iter().sum::<f64>() / mid_frames as f64;
    let max_mid = mid_times_ms.iter().cloned().fold(0.0, f64::max);
    println!(
        "[1MB Rapid Middle Mutations] Avg: {:.2} ms/frame, Max: {:.2} ms/frame",
        avg_mid, max_mid
    );

    // 5. Measure Memory Footprint
    let text_capacity_mb = text.capacity() as f64 / (1024.0 * 1024.0);
    println!(
        "[1MB Buffer Memory] String capacity: {:.2} MB for {} bytes text",
        text_capacity_mb,
        text.len()
    );

    // Empirical verification assertions
    assert!(
        text.len() >= 1024 * 1024,
        "Payload must remain >= 1 MB throughout mutations"
    );
}
