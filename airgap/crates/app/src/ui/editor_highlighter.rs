use egui::text::{LayoutJob, TextFormat};
use egui::{vec2, Color32, FontId, Galley, Ui};
use std::ops::Range;
use std::sync::Arc;

/// 64-bit FNV-1a fast string hasher.
pub fn fast_hash_str(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[derive(Debug, Clone, Copy)]
pub struct EditorPalette {
    pub normal: Color32,
    pub h1: Color32,
    pub h2: Color32,
    pub h3: Color32,
    pub h4_6: Color32,
    pub marker_muted: Color32,
    pub bold: Color32,
    pub italic: Color32,
    pub code_inline: Color32,
    pub code_inline_bg: Color32,
    pub link: Color32,
    pub list_bullet: Color32,
    pub task_unchecked: Color32,
    pub task_checked: Color32,
    pub blockquote: Color32,
    pub table_pipe: Color32,
    pub hr: Color32,
    pub code_fence: Color32,
    pub code_bg: Color32,
    pub code_text: Color32,
    pub match_bg: Color32,
    pub active_match_bg: Color32,
}

impl EditorPalette {
    pub fn dark() -> Self {
        Self {
            normal: Color32::from_rgb(228, 228, 231),
            h1: Color32::from_rgb(96, 165, 250),
            h2: Color32::from_rgb(56, 189, 248),
            h3: Color32::from_rgb(52, 211, 153),
            h4_6: Color32::from_rgb(251, 191, 36),
            marker_muted: Color32::from_rgb(113, 113, 122),
            bold: Color32::WHITE,
            italic: Color32::from_rgb(212, 212, 216),
            code_inline: Color32::from_rgb(244, 114, 182),
            code_inline_bg: Color32::from_rgba_premultiplied(244, 114, 182, 30),
            link: Color32::from_rgb(96, 165, 250),
            list_bullet: Color32::from_rgb(56, 189, 248),
            task_unchecked: Color32::from_rgb(245, 158, 11),
            task_checked: Color32::from_rgb(34, 197, 94),
            blockquote: Color32::from_rgb(147, 197, 253),
            table_pipe: Color32::from_rgb(82, 82, 91),
            hr: Color32::from_rgb(82, 82, 91),
            code_fence: Color32::from_rgb(113, 113, 122),
            code_bg: Color32::from_rgb(15, 15, 18),
            code_text: Color32::from_rgb(212, 212, 216),
            match_bg: Color32::from_rgba_unmultiplied(234, 179, 8, 80),
            active_match_bg: Color32::from_rgba_unmultiplied(249, 115, 22, 140),
        }
    }

    pub fn light() -> Self {
        Self {
            normal: Color32::from_rgb(24, 24, 27),
            h1: Color32::from_rgb(29, 78, 216),
            h2: Color32::from_rgb(2, 132, 199),
            h3: Color32::from_rgb(5, 150, 105),
            h4_6: Color32::from_rgb(217, 119, 6),
            marker_muted: Color32::from_rgb(161, 161, 170),
            bold: Color32::BLACK,
            italic: Color32::from_rgb(39, 39, 42),
            code_inline: Color32::from_rgb(190, 24, 93),
            code_inline_bg: Color32::from_rgba_premultiplied(190, 24, 93, 25),
            link: Color32::from_rgb(29, 78, 216),
            list_bullet: Color32::from_rgb(2, 132, 199),
            task_unchecked: Color32::from_rgb(217, 119, 6),
            task_checked: Color32::from_rgb(22, 163, 74),
            blockquote: Color32::from_rgb(37, 99, 235),
            table_pipe: Color32::from_rgb(212, 212, 216),
            hr: Color32::from_rgb(212, 212, 216),
            code_fence: Color32::from_rgb(161, 161, 170),
            code_bg: Color32::from_rgb(244, 244, 245),
            code_text: Color32::from_rgb(39, 39, 42),
            match_bg: Color32::from_rgba_unmultiplied(234, 179, 8, 80),
            active_match_bg: Color32::from_rgba_unmultiplied(249, 115, 22, 140),
        }
    }
}

/// Cached metadata for a single logical line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CachedLine {
    pub hash: u64,
    pub row_count: usize,
}

pub struct EditorHighlighter {
    cached_galley: Option<Arc<Galley>>,
    deferred_drop_holder: Option<Arc<Galley>>,
    cached_lines: Vec<CachedLine>,
    cached_text_hash: u64,
    cached_wrap_width: f32,
    cached_is_dark: bool,
    cached_font_size: f32,
    cached_matches_hash: u64,
}

impl Default for EditorHighlighter {
    fn default() -> Self {
        Self {
            cached_galley: None,
            deferred_drop_holder: None,
            cached_lines: Vec::new(),
            cached_text_hash: 0,
            cached_wrap_width: 0.0,
            cached_is_dark: true,
            cached_font_size: 14.0,
            cached_matches_hash: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct LayoutOptions<'a> {
    pub wrap_width: f32,
    pub is_dark: bool,
    pub font_size: f32,
    pub find_matches: &'a [Range<usize>],
    pub active_match: usize,
}

impl EditorHighlighter {
    pub fn layout(&mut self, ui: &Ui, text: &str, opts: &LayoutOptions<'_>) -> Arc<Galley> {
        let text_hash = fast_hash_str(text);

        // Compute match hash
        let mut matches_hash = opts.find_matches.len() as u64;
        if let Some(m) = opts.find_matches.get(opts.active_match) {
            matches_hash = matches_hash
                .wrapping_mul(31)
                .wrapping_add(m.start as u64)
                .wrapping_add(m.end as u64);
        }

        let options_unchanged = (self.cached_wrap_width - opts.wrap_width).abs() < 0.1
            && self.cached_is_dark == opts.is_dark
            && (self.cached_font_size - opts.font_size).abs() < 0.1
            && self.cached_matches_hash == matches_hash;

        // Tier 1: Fast memoization check (idle frame: 0.00ms)
        if let Some(galley) = &self.cached_galley {
            if self.cached_text_hash == text_hash && options_unchanged {
                return galley.clone();
            }
        }

        let palette = if opts.is_dark {
            EditorPalette::dark()
        } else {
            EditorPalette::light()
        };

        // Tier 2: Differential line-level update (<15ms on 1MB documents)
        if options_unchanged && !self.cached_lines.is_empty() && self.cached_galley.is_some() {
            if let Some(galley) = self.try_differential_update(ui, text, opts, &palette) {
                self.cached_text_hash = text_hash;
                return galley;
            }
        }

        // Tier 3: Full layout (<500ms on 1MB documents)
        self.full_layout(ui, text, opts, text_hash, matches_hash, &palette)
    }

    fn try_differential_update(
        &mut self,
        ui: &Ui,
        text: &str,
        opts: &LayoutOptions<'_>,
        palette: &EditorPalette,
    ) -> Option<Arc<Galley>> {
        let text_lines: Vec<&str> = text.split_inclusive('\n').collect();
        let old_count = self.cached_lines.len();
        let new_count = text_lines.len();

        // 1. Two-pointer scan from start
        let mut start = 0;
        while start < old_count && start < new_count {
            if fast_hash_str(text_lines[start]) != self.cached_lines[start].hash {
                break;
            }
            start += 1;
        }

        let num_changed_new;
        let num_changed_old;
        let end_old;
        let end_new;

        // O(1) check for single-line typing: if line count is identical and line after start is unchanged
        if old_count == new_count
            && start < old_count
            && (start + 1 == old_count
                || fast_hash_str(text_lines[start + 1]) == self.cached_lines[start + 1].hash)
        {
            end_old = start + 1;
            end_new = start + 1;
            num_changed_new = 1;
            num_changed_old = 1;
        } else {
            // General two-pointer scan from end for multi-line edits / Enter / deletions
            let mut eo = old_count;
            let mut en = new_count;
            while eo > start && en > start {
                if fast_hash_str(text_lines[en - 1]) != self.cached_lines[eo - 1].hash {
                    break;
                }
                eo -= 1;
                en -= 1;
            }
            end_old = eo;
            end_new = en;
            num_changed_new = end_new - start;
            num_changed_old = end_old - start;
        }

        // Fall back to full layout for large multi-line edits/pastes (>16 lines)
        if num_changed_new > 16 || num_changed_old > 16 {
            return None;
        }

        // Compute byte offset of start line
        let mut byte_offset = 0;
        for line in &text_lines[..start] {
            byte_offset += line.len();
        }

        // Layout only changed lines
        let mut new_cached_lines = Vec::with_capacity(num_changed_new);
        let mut new_rows = Vec::with_capacity(num_changed_new);

        for &line in &text_lines[start..end_new] {
            let line_len = line.len();
            let line_start = byte_offset;
            let line_hash = fast_hash_str(line);

            let mut line_job = LayoutJob::default();
            line_job.wrap.max_width = opts.wrap_width;

            let fmt = self.get_line_format(line, opts.font_size, palette);
            self.append_spans_with_find(
                &mut line_job,
                line,
                line_start,
                fmt,
                opts.find_matches,
                opts.active_match,
                palette,
            );

            let line_galley = ui.fonts(|f| f.layout_job(line_job));
            let mut valid_rows: Vec<egui::epaint::text::Row> = line_galley.rows.to_vec();
            if line.ends_with('\n')
                && valid_rows.len() > 1
                && valid_rows.last().is_some_and(|r| r.glyphs.is_empty())
            {
                valid_rows.pop();
            }
            let row_count = valid_rows.len();
            new_cached_lines.push(CachedLine {
                hash: line_hash,
                row_count,
            });
            for row in valid_rows {
                new_rows.push(row);
            }

            byte_offset += line_len;
        }

        // Ensure target galley exists and can be uniquely modified
        let target_arc = self.cached_galley.as_mut().unwrap();
        if Arc::strong_count(target_arc) > 1 {
            *target_arc = Arc::new((**target_arc).clone());
        }
        let galley_mut = Arc::get_mut(target_arc).unwrap();

        // Map line range to row indices
        let mut row_start = 0;
        for line in &self.cached_lines[..start] {
            row_start += line.row_count;
        }

        let mut row_old_count = 0;
        for line in &self.cached_lines[start..end_old] {
            row_old_count += line.row_count;
        }
        let row_end_old = row_start + row_old_count;

        if row_end_old > galley_mut.rows.len() {
            return None;
        }

        // Fast path: Single-line edit with unchanged row height (common typing path)
        if num_changed_new == 1
            && num_changed_old == 1
            && new_rows.len() == 1
            && row_old_count == 1
            && row_start < galley_mut.rows.len()
        {
            let old_y = galley_mut.rows[row_start].rect.min.y;
            let old_height = galley_mut.rows[row_start].rect.height();
            let mut r = new_rows[0].clone();
            let new_height = r.rect.height();

            r.rect.min.y = old_y;
            r.rect.max.y = old_y + new_height;
            for g in &mut r.glyphs {
                g.pos.y += old_y;
            }
            for v in &mut r.visuals.mesh.vertices {
                v.pos.y += old_y;
            }
            r.visuals.mesh_bounds = r.visuals.mesh_bounds.translate(vec2(0.0, old_y));

            if (new_height - old_height).abs() < 0.01 {
                let old_v = galley_mut.rows[row_start].visuals.mesh.vertices.len();
                let old_i = galley_mut.rows[row_start].visuals.mesh.indices.len();
                let new_v = r.visuals.mesh.vertices.len();
                let new_i = r.visuals.mesh.indices.len();

                galley_mut.rows[row_start] = r;
                self.cached_lines[start] = new_cached_lines[0];

                galley_mut.num_vertices = (galley_mut.num_vertices + new_v) - old_v;
                galley_mut.num_indices = (galley_mut.num_indices + new_i) - old_i;

                return Some(target_arc.clone());
            }
        }

        // General path: Multi-line / newline / height-changing edit
        let old_y_start = if row_start < galley_mut.rows.len() {
            galley_mut.rows[row_start].rect.min.y
        } else {
            0.0
        };

        let old_height: f32 = galley_mut.rows[row_start..row_end_old]
            .iter()
            .map(|r| r.rect.height())
            .sum();

        let mut curr_y = old_y_start;
        for r in &mut new_rows {
            let h = r.rect.height();
            let dy = curr_y;
            r.rect.min.y = curr_y;
            r.rect.max.y = curr_y + h;
            for g in &mut r.glyphs {
                g.pos.y += dy;
            }
            for v in &mut r.visuals.mesh.vertices {
                v.pos.y += dy;
            }
            r.visuals.mesh_bounds = r.visuals.mesh_bounds.translate(vec2(0.0, dy));
            curr_y += h;
        }

        let new_height = curr_y - old_y_start;
        let delta_y = new_height - old_height;

        let old_v: usize = galley_mut.rows[row_start..row_end_old]
            .iter()
            .map(|r| r.visuals.mesh.vertices.len())
            .sum();
        let old_i: usize = galley_mut.rows[row_start..row_end_old]
            .iter()
            .map(|r| r.visuals.mesh.indices.len())
            .sum();
        let new_v: usize = new_rows.iter().map(|r| r.visuals.mesh.vertices.len()).sum();
        let new_i: usize = new_rows.iter().map(|r| r.visuals.mesh.indices.len()).sum();

        let num_new_rows = new_rows.len();
        galley_mut.rows.splice(row_start..row_end_old, new_rows);
        self.cached_lines.splice(start..end_old, new_cached_lines);

        galley_mut.num_vertices = (galley_mut.num_vertices + new_v) - old_v;
        galley_mut.num_indices = (galley_mut.num_indices + new_i) - old_i;

        if delta_y.abs() >= 0.01 {
            for r in &mut galley_mut.rows[row_start + num_new_rows..] {
                r.rect.min.y += delta_y;
                r.rect.max.y += delta_y;
                for g in &mut r.glyphs {
                    g.pos.y += delta_y;
                }
                for v in &mut r.visuals.mesh.vertices {
                    v.pos.y += delta_y;
                }
                r.visuals.mesh_bounds = r.visuals.mesh_bounds.translate(vec2(0.0, delta_y));
            }
            galley_mut.rect.max.y += delta_y;
        }

        Some(target_arc.clone())
    }

    fn full_layout(
        &mut self,
        ui: &Ui,
        text: &str,
        opts: &LayoutOptions<'_>,
        text_hash: u64,
        matches_hash: u64,
        palette: &EditorPalette,
    ) -> Arc<Galley> {
        let job = self.build_job(
            text,
            opts.wrap_width,
            palette,
            opts.font_size,
            opts.find_matches,
            opts.active_match,
        );

        let epaint_galley = ui.fonts(|f| f.layout_job(job));

        // Retain epaint_galley in deferred holder so epaint's cache flush does not trigger 25ms HeapFree stall
        self.deferred_drop_holder = Some(epaint_galley.clone());

        let text_lines: Vec<&str> = text.split_inclusive('\n').collect();
        let mut cached_lines = Vec::with_capacity(text_lines.len());
        if epaint_galley.rows.len() == text_lines.len() {
            for l in &text_lines {
                cached_lines.push(CachedLine {
                    hash: fast_hash_str(l),
                    row_count: 1,
                });
            }
        } else {
            let mut row_idx = 0;
            for l in &text_lines {
                let hash = fast_hash_str(l);
                let start_r = row_idx;
                while row_idx < epaint_galley.rows.len() {
                    let ends_nl = epaint_galley.rows[row_idx].ends_with_newline;
                    row_idx += 1;
                    if ends_nl {
                        break;
                    }
                }
                let count = (row_idx - start_r).max(1);
                cached_lines.push(CachedLine {
                    hash,
                    row_count: count,
                });
            }
        }

        self.cached_lines = cached_lines;
        let independent_galley = Arc::new((*epaint_galley).clone());
        self.cached_galley = Some(independent_galley.clone());
        self.cached_text_hash = text_hash;
        self.cached_wrap_width = opts.wrap_width;
        self.cached_is_dark = opts.is_dark;
        self.cached_font_size = opts.font_size;
        self.cached_matches_hash = matches_hash;

        independent_galley
    }

    fn get_line_format(&self, line: &str, font_size: f32, palette: &EditorPalette) -> TextFormat {
        let mono = FontId::monospace(font_size);
        let trimmed = line.trim_end_matches(['\r', '\n']);

        if trimmed.starts_with("```") {
            TextFormat {
                font_id: mono,
                color: palette.code_fence,
                background: palette.code_bg,
                ..Default::default()
            }
        } else if trimmed.starts_with("###### ")
            || trimmed.starts_with("##### ")
            || trimmed.starts_with("#### ")
        {
            TextFormat {
                font_id: FontId::monospace(font_size),
                color: palette.h4_6,
                ..Default::default()
            }
        } else if trimmed.starts_with("### ") {
            TextFormat {
                font_id: FontId::monospace(font_size),
                color: palette.h3,
                ..Default::default()
            }
        } else if trimmed.starts_with("## ") {
            TextFormat {
                font_id: FontId::monospace(font_size),
                color: palette.h2,
                ..Default::default()
            }
        } else if trimmed.starts_with("# ") {
            TextFormat {
                font_id: FontId::monospace(font_size),
                color: palette.h1,
                ..Default::default()
            }
        } else if trimmed.starts_with('>') {
            TextFormat {
                font_id: mono,
                color: palette.blockquote,
                italics: true,
                ..Default::default()
            }
        } else if trimmed.starts_with("- [ ] ") || trimmed.starts_with("* [ ] ") {
            TextFormat {
                font_id: mono,
                color: palette.task_unchecked,
                ..Default::default()
            }
        } else if trimmed.starts_with("- [x] ")
            || trimmed.starts_with("- [X] ")
            || trimmed.starts_with("* [x] ")
        {
            TextFormat {
                font_id: mono,
                color: palette.task_checked,
                ..Default::default()
            }
        } else if trimmed.starts_with("---") || trimmed.starts_with("***") {
            TextFormat {
                font_id: mono,
                color: palette.hr,
                ..Default::default()
            }
        } else {
            TextFormat {
                font_id: mono,
                color: palette.normal,
                ..Default::default()
            }
        }
    }

    fn build_job(
        &self,
        text: &str,
        wrap_width: f32,
        palette: &EditorPalette,
        font_size: f32,
        find_matches: &[Range<usize>],
        active_match: usize,
    ) -> LayoutJob {
        let mut job = LayoutJob::default();
        job.wrap.max_width = wrap_width;

        let mono = FontId::monospace(font_size);
        let mut in_code_fence = false;
        let mut byte_offset = 0;

        for line in text.split_inclusive('\n') {
            let line_len = line.len();
            let line_start = byte_offset;
            let line_end = line_start + line_len;
            let trimmed = line.trim_end_matches(['\r', '\n']);

            // Code fence toggle
            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                let fmt = TextFormat {
                    font_id: mono.clone(),
                    color: palette.code_fence,
                    background: palette.code_bg,
                    ..Default::default()
                };
                self.append_spans_with_find(
                    &mut job,
                    line,
                    line_start,
                    fmt,
                    find_matches,
                    active_match,
                    palette,
                );
                byte_offset = line_end;
                continue;
            }

            if in_code_fence {
                let fmt = TextFormat {
                    font_id: mono.clone(),
                    color: palette.code_text,
                    background: palette.code_bg,
                    ..Default::default()
                };
                self.append_spans_with_find(
                    &mut job,
                    line,
                    line_start,
                    fmt,
                    find_matches,
                    active_match,
                    palette,
                );
                byte_offset = line_end;
                continue;
            }

            let fmt = self.get_line_format(line, font_size, palette);
            self.append_spans_with_find(
                &mut job,
                line,
                line_start,
                fmt,
                find_matches,
                active_match,
                palette,
            );

            byte_offset = line_end;
        }

        job
    }

    #[allow(clippy::too_many_arguments)]
    fn append_spans_with_find(
        &self,
        job: &mut LayoutJob,
        text: &str,
        line_start: usize,
        base_fmt: TextFormat,
        find_matches: &[Range<usize>],
        active_match: usize,
        palette: &EditorPalette,
    ) {
        let line_end = line_start + text.len();

        // 1. Filter and validate match ranges against text boundaries and UTF-8 char boundaries
        let mut match_ranges: Vec<(Range<usize>, bool)> = Vec::new();
        for (idx, mat) in find_matches.iter().enumerate() {
            if mat.end > line_start && mat.start < line_end {
                let clamped_start = mat.start.max(line_start) - line_start;
                let clamped_end = mat.end.min(line_end) - line_start;

                // CRITICAL DEFECT FIX: Verify both endpoints land on valid UTF-8 character boundaries.
                // Stale find matches from a pre-keystroke buffer can point mid-codepoint in mutated CJK/emojis.
                if clamped_start < clamped_end
                    && clamped_end <= text.len()
                    && text.is_char_boundary(clamped_start)
                    && text.is_char_boundary(clamped_end)
                {
                    match_ranges.push((clamped_start..clamped_end, idx == active_match));
                }
            }
        }

        if match_ranges.is_empty() {
            job.append(text, 0.0, base_fmt);
            return;
        }

        // 2. Sort match ranges monotonically to guarantee safe progressive slicing
        match_ranges.sort_by_key(|(r, _)| (r.start, r.end));

        let mut curr = 0;
        for (range, is_active) in match_ranges {
            // Skip stale overlapping matches that start before current progression
            if range.start < curr {
                continue;
            }
            if range.start > curr {
                job.append(&text[curr..range.start], 0.0, base_fmt.clone());
            }

            let mut match_fmt = base_fmt.clone();
            match_fmt.background = if is_active {
                palette.active_match_bg
            } else {
                palette.match_bg
            };
            job.append(&text[range.start..range.end], 0.0, match_fmt);
            curr = range.end;
        }

        if curr < text.len() {
            job.append(&text[curr..], 0.0, base_fmt);
        }
    }
}
