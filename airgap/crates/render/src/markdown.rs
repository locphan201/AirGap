use crate::model::{
    Block, CalloutKind, ColumnAlignment, DocModel, InlineSpan, InlineStyle, ListItem, SourceSpan,
};
use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use std::collections::HashMap;

/// Maps byte offsets to 1-indexed lines and source spans.
#[derive(Debug, Clone)]
pub struct LineIndex {
    line_starts: Vec<usize>,
    total_bytes: usize,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (idx, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(idx + 1);
            }
        }
        Self {
            line_starts,
            total_bytes: text.len(),
        }
    }

    pub fn offset_to_line(&self, byte_offset: usize) -> usize {
        let clamped = byte_offset.min(self.total_bytes);
        match self.line_starts.binary_search(&clamped) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        }
    }

    pub fn byte_range_to_span(&self, range: &std::ops::Range<usize>) -> SourceSpan {
        let start_line = self.offset_to_line(range.start);
        let end_line = if range.end > range.start {
            self.offset_to_line(range.end - 1).max(start_line)
        } else {
            start_line
        };
        SourceSpan::new(start_line, end_line, range.start, range.end)
    }

    pub fn total_lines(&self) -> usize {
        self.line_starts.len()
    }
}

/// Generates a URL-friendly anchor slug from heading text.
pub fn generate_slug(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    let mut prev_was_hyphen = false;

    for c in text.chars() {
        if c.is_alphanumeric() {
            for lc in c.to_lowercase() {
                slug.push(lc);
            }
            prev_was_hyphen = false;
        } else if (c.is_whitespace() || c == '-' || c == '_')
            && !prev_was_hyphen
            && !slug.is_empty()
        {
            slug.push('-');
            prev_was_hyphen = true;
        }
    }

    if slug.ends_with('-') {
        slug.pop();
    }

    slug
}

#[derive(Default)]
struct StyleState {
    bold: usize,
    italic: usize,
    strikethrough: usize,
}

impl StyleState {
    fn to_inline_span(&self, text: &str) -> InlineSpan {
        if self.bold > 0 && self.italic > 0 {
            InlineSpan::StyledText {
                text: text.to_string(),
                style: InlineStyle {
                    bold: true,
                    italic: true,
                    strikethrough: self.strikethrough > 0,
                    code: false,
                },
            }
        } else if self.bold > 0 {
            if self.strikethrough > 0 {
                InlineSpan::StyledText {
                    text: text.to_string(),
                    style: InlineStyle {
                        bold: true,
                        italic: false,
                        strikethrough: true,
                        code: false,
                    },
                }
            } else {
                InlineSpan::Bold(text.to_string())
            }
        } else if self.italic > 0 {
            if self.strikethrough > 0 {
                InlineSpan::StyledText {
                    text: text.to_string(),
                    style: InlineStyle {
                        bold: false,
                        italic: true,
                        strikethrough: true,
                        code: false,
                    },
                }
            } else {
                InlineSpan::Italic(text.to_string())
            }
        } else if self.strikethrough > 0 {
            InlineSpan::Strikethrough(text.to_string())
        } else {
            InlineSpan::Text(text.to_string())
        }
    }
}

enum LinkOrImage {
    Link {
        url: String,
        title: Option<String>,
        text: String,
    },
    Image {
        url: String,
        title: Option<String>,
        alt: String,
    },
}

struct ActiveHeading {
    level: u8,
    id: Option<String>,
    classes: Vec<String>,
    inlines: Vec<InlineSpan>,
    plain: String,
    start_byte: usize,
}

struct ActiveBlockQuote {
    callout: Option<CalloutKind>,
    inlines: Vec<InlineSpan>,
    lines: Vec<String>,
    start_byte: usize,
}

struct ActiveList {
    is_ordered: bool,
    start_num: Option<u64>,
    items: Vec<ListItem>,
    start_byte: usize,
}

struct ActiveItem {
    task_status: Option<bool>,
    inlines: Vec<InlineSpan>,
    plain: String,
    start_byte: usize,
}

struct ActiveTable {
    alignments: Vec<ColumnAlignment>,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    header_inlines: Vec<Vec<InlineSpan>>,
    row_inlines: Vec<Vec<Vec<InlineSpan>>>,
    start_byte: usize,
}

/// Parses CommonMark + GFM markdown text into our intermediate DocModel.
/// Detects ```mermaid blocks and tags them as Block::MermaidBlock.
pub fn parse_markdown(markdown_text: &str) -> DocModel {
    let line_index = LineIndex::new(markdown_text);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_GFM);

    let parser = Parser::new_ext(markdown_text, options).into_offset_iter();

    let mut blocks = Vec::new();
    let mut footnotes = HashMap::new();

    let mut style = StyleState::default();
    let mut link_stack: Vec<LinkOrImage> = Vec::new();

    let mut in_code_block: Option<(String, String, usize)> = None;
    let mut in_heading: Option<ActiveHeading> = None;
    let mut in_paragraph: Option<(Vec<InlineSpan>, usize)> = None;
    let mut in_blockquote: Option<ActiveBlockQuote> = None;
    let mut in_list: Option<ActiveList> = None;
    let mut in_item: Option<ActiveItem> = None;
    let mut in_table: Option<ActiveTable> = None;
    let mut in_table_row: Option<(Vec<String>, Vec<Vec<InlineSpan>>)> = None;
    let mut in_table_cell: Option<(Vec<InlineSpan>, String)> = None;
    let mut in_footnote: Option<(String, Vec<InlineSpan>, usize)> = None;

    for (event, range) in parser {
        match event {
            // Code Blocks
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(l) => l.trim().to_lowercase(),
                    CodeBlockKind::Indented => String::new(),
                };
                in_code_block = Some((lang, String::new(), range.start));
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((lang, code, start_byte)) = in_code_block.take() {
                    let span = line_index.byte_range_to_span(&(start_byte..range.end));
                    if lang == "mermaid" {
                        blocks.push(Block::MermaidBlock {
                            source: code,
                            line: span.start_line,
                            span,
                        });
                    } else {
                        blocks.push(Block::CodeBlock {
                            language: lang,
                            code,
                            line: span.start_line,
                            span,
                        });
                    }
                }
            }

            // Headings
            Event::Start(Tag::Heading {
                level, id, classes, ..
            }) => {
                let lvl = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                let id_str = id.map(|s| s.to_string());
                let classes_vec = classes.into_iter().map(|s| s.to_string()).collect();
                in_heading = Some(ActiveHeading {
                    level: lvl,
                    id: id_str,
                    classes: classes_vec,
                    inlines: Vec::new(),
                    plain: String::new(),
                    start_byte: range.start,
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(h) = in_heading.take() {
                    let span = line_index.byte_range_to_span(&(h.start_byte..range.end));
                    let heading_text = h.plain.trim().to_string();
                    let id = h.id.or_else(|| {
                        let slug = generate_slug(&heading_text);
                        if slug.is_empty() {
                            None
                        } else {
                            Some(slug)
                        }
                    });
                    blocks.push(Block::Heading {
                        level: h.level,
                        text: heading_text,
                        inlines: h.inlines,
                        id,
                        classes: h.classes,
                        line: span.start_line,
                        span,
                    });
                }
            }

            // Paragraphs
            Event::Start(Tag::Paragraph) => {
                in_paragraph = Some((Vec::new(), range.start));
            }
            Event::End(TagEnd::Paragraph) => {
                if let Some((inlines, start_byte)) = in_paragraph.take() {
                    let span = line_index.byte_range_to_span(&(start_byte..range.end));
                    if let Some(item) = in_item.as_mut() {
                        item.inlines.extend(inlines.clone());
                        for inline in &inlines {
                            item.plain.push_str(inline.plain_text());
                        }
                    } else if let Some(bq) = in_blockquote.as_mut() {
                        let text: String = inlines.iter().map(|i| i.plain_text()).collect();
                        bq.lines.push(text);
                        bq.inlines.extend(inlines);
                    } else {
                        blocks.push(Block::Paragraph {
                            inlines,
                            line: span.start_line,
                            span,
                        });
                    }
                }
            }

            // BlockQuotes
            Event::Start(Tag::BlockQuote(kind)) => {
                let callout = kind.map(|k| match k {
                    BlockQuoteKind::Note => CalloutKind::Note,
                    BlockQuoteKind::Tip => CalloutKind::Tip,
                    BlockQuoteKind::Important => CalloutKind::Important,
                    BlockQuoteKind::Warning => CalloutKind::Warning,
                    BlockQuoteKind::Caution => CalloutKind::Caution,
                });
                in_blockquote = Some(ActiveBlockQuote {
                    callout,
                    inlines: Vec::new(),
                    lines: Vec::new(),
                    start_byte: range.start,
                });
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                if let Some(bq) = in_blockquote.take() {
                    let span = line_index.byte_range_to_span(&(bq.start_byte..range.end));
                    blocks.push(Block::BlockQuote {
                        lines: bq.lines,
                        inlines: bq.inlines,
                        callout: bq.callout,
                        line: span.start_line,
                        span,
                    });
                }
            }

            // Lists & Items
            Event::Start(Tag::List(start_num)) => {
                let is_ordered = start_num.is_some();
                in_list = Some(ActiveList {
                    is_ordered,
                    start_num,
                    items: Vec::new(),
                    start_byte: range.start,
                });
            }
            Event::End(TagEnd::List(_)) => {
                if let Some(l) = in_list.take() {
                    let span = line_index.byte_range_to_span(&(l.start_byte..range.end));
                    blocks.push(Block::List {
                        items: l.items,
                        is_ordered: l.is_ordered,
                        start_number: l.start_num,
                        line: span.start_line,
                        span,
                    });
                }
            }

            Event::Start(Tag::Item) => {
                in_item = Some(ActiveItem {
                    task_status: None,
                    inlines: Vec::new(),
                    plain: String::new(),
                    start_byte: range.start,
                });
            }
            Event::End(TagEnd::Item) => {
                if let Some(item_data) = in_item.take() {
                    let span = line_index.byte_range_to_span(&(item_data.start_byte..range.end));
                    let item = ListItem {
                        text: item_data.plain.trim().to_string(),
                        inlines: item_data.inlines,
                        task_status: item_data.task_status,
                        span,
                    };
                    if let Some(list) = in_list.as_mut() {
                        list.items.push(item);
                    }
                }
            }
            Event::TaskListMarker(checked) => {
                if let Some(item) = in_item.as_mut() {
                    item.task_status = Some(checked);
                }
            }

            // Tables
            Event::Start(Tag::Table(alignments)) => {
                let aligns = alignments
                    .into_iter()
                    .map(|a| match a {
                        Alignment::None => ColumnAlignment::None,
                        Alignment::Left => ColumnAlignment::Left,
                        Alignment::Center => ColumnAlignment::Center,
                        Alignment::Right => ColumnAlignment::Right,
                    })
                    .collect();
                in_table = Some(ActiveTable {
                    alignments: aligns,
                    headers: Vec::new(),
                    rows: Vec::new(),
                    header_inlines: Vec::new(),
                    row_inlines: Vec::new(),
                    start_byte: range.start,
                });
            }
            Event::End(TagEnd::Table) => {
                if let Some(tbl) = in_table.take() {
                    let span = line_index.byte_range_to_span(&(tbl.start_byte..range.end));
                    blocks.push(Block::Table {
                        headers: tbl.headers,
                        rows: tbl.rows,
                        alignments: tbl.alignments,
                        header_inlines: tbl.header_inlines,
                        row_inlines: tbl.row_inlines,
                        line: span.start_line,
                        span,
                    });
                }
            }
            Event::Start(Tag::TableHead) => {
                in_table_row = Some((Vec::new(), Vec::new()));
            }
            Event::End(TagEnd::TableHead) => {
                if let (Some(tbl), Some((row_cells, row_cell_inlines))) =
                    (in_table.as_mut(), in_table_row.take())
                {
                    tbl.headers = row_cells;
                    tbl.header_inlines = row_cell_inlines;
                }
            }
            Event::Start(Tag::TableRow) => {
                in_table_row = Some((Vec::new(), Vec::new()));
            }
            Event::End(TagEnd::TableRow) => {
                if let (Some(tbl), Some((row_cells, row_cell_inlines))) =
                    (in_table.as_mut(), in_table_row.take())
                {
                    tbl.rows.push(row_cells);
                    tbl.row_inlines.push(row_cell_inlines);
                }
            }
            Event::Start(Tag::TableCell) => {
                in_table_cell = Some((Vec::new(), String::new()));
            }
            Event::End(TagEnd::TableCell) => {
                if let Some((cell_inlines, plain)) = in_table_cell.take() {
                    if let Some((row_cells, row_cell_inlines)) = in_table_row.as_mut() {
                        row_cells.push(plain.trim().to_string());
                        row_cell_inlines.push(cell_inlines);
                    }
                }
            }

            // Footnote Definitions
            Event::Start(Tag::FootnoteDefinition(label)) => {
                in_footnote = Some((label.to_string(), Vec::new(), range.start));
            }
            Event::End(TagEnd::FootnoteDefinition) => {
                if let Some((label, inlines, start_byte)) = in_footnote.take() {
                    let span = line_index.byte_range_to_span(&(start_byte..range.end));
                    footnotes.insert(label.clone(), inlines.clone());
                    blocks.push(Block::FootnoteDefinition {
                        label,
                        inlines,
                        line: span.start_line,
                        span,
                    });
                }
            }

            // Inline formatting tags
            Event::Start(Tag::Emphasis) => {
                style.italic += 1;
            }
            Event::End(TagEnd::Emphasis) => {
                style.italic = style.italic.saturating_sub(1);
            }
            Event::Start(Tag::Strong) => {
                style.bold += 1;
            }
            Event::End(TagEnd::Strong) => {
                style.bold = style.bold.saturating_sub(1);
            }
            Event::Start(Tag::Strikethrough) => {
                style.strikethrough += 1;
            }
            Event::End(TagEnd::Strikethrough) => {
                style.strikethrough = style.strikethrough.saturating_sub(1);
            }
            Event::Start(Tag::Link {
                dest_url, title, ..
            }) => {
                link_stack.push(LinkOrImage::Link {
                    url: dest_url.to_string(),
                    title: if title.is_empty() {
                        None
                    } else {
                        Some(title.to_string())
                    },
                    text: String::new(),
                });
            }
            Event::End(TagEnd::Link) => {
                if let Some(LinkOrImage::Link { url, title, text }) = link_stack.pop() {
                    let inline = InlineSpan::Link { text, url, title };
                    dispatch_inline(
                        inline,
                        &mut in_paragraph,
                        &mut in_heading,
                        &mut in_item,
                        &mut in_table_cell,
                        &mut in_footnote,
                        &mut link_stack,
                    );
                }
            }
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                link_stack.push(LinkOrImage::Image {
                    url: dest_url.to_string(),
                    title: if title.is_empty() {
                        None
                    } else {
                        Some(title.to_string())
                    },
                    alt: String::new(),
                });
            }
            Event::End(TagEnd::Image) => {
                if let Some(LinkOrImage::Image { url, title, alt }) = link_stack.pop() {
                    let inline = InlineSpan::Image {
                        alt,
                        url,
                        title,
                        resolved_path: None,
                    };
                    dispatch_inline(
                        inline,
                        &mut in_paragraph,
                        &mut in_heading,
                        &mut in_item,
                        &mut in_table_cell,
                        &mut in_footnote,
                        &mut link_stack,
                    );
                }
            }

            // Content Events
            Event::Text(text) => {
                if let Some((_, buffer, _)) = in_code_block.as_mut() {
                    buffer.push_str(&text);
                } else if let Some(last_link) = link_stack.last_mut() {
                    match last_link {
                        LinkOrImage::Link {
                            text: link_text, ..
                        } => link_text.push_str(&text),
                        LinkOrImage::Image { alt, .. } => alt.push_str(&text),
                    }
                } else {
                    let inline = style.to_inline_span(&text);
                    dispatch_inline(
                        inline,
                        &mut in_paragraph,
                        &mut in_heading,
                        &mut in_item,
                        &mut in_table_cell,
                        &mut in_footnote,
                        &mut link_stack,
                    );
                }
            }
            Event::Code(code) => {
                let inline = InlineSpan::Code(code.to_string());
                dispatch_inline(
                    inline,
                    &mut in_paragraph,
                    &mut in_heading,
                    &mut in_item,
                    &mut in_table_cell,
                    &mut in_footnote,
                    &mut link_stack,
                );
            }
            Event::SoftBreak => {
                dispatch_inline(
                    InlineSpan::SoftBreak,
                    &mut in_paragraph,
                    &mut in_heading,
                    &mut in_item,
                    &mut in_table_cell,
                    &mut in_footnote,
                    &mut link_stack,
                );
            }
            Event::HardBreak => {
                dispatch_inline(
                    InlineSpan::HardBreak,
                    &mut in_paragraph,
                    &mut in_heading,
                    &mut in_item,
                    &mut in_table_cell,
                    &mut in_footnote,
                    &mut link_stack,
                );
            }
            Event::FootnoteReference(label) => {
                dispatch_inline(
                    InlineSpan::FootnoteRef(label.to_string()),
                    &mut in_paragraph,
                    &mut in_heading,
                    &mut in_item,
                    &mut in_table_cell,
                    &mut in_footnote,
                    &mut link_stack,
                );
            }
            Event::Rule => {
                let span = line_index.byte_range_to_span(&range);
                blocks.push(Block::ThematicBreak {
                    line: span.start_line,
                    span,
                });
            }
            _ => {}
        }
    }

    DocModel {
        blocks,
        footnotes,
        total_lines: line_index.total_lines(),
    }
}

fn dispatch_inline(
    inline: InlineSpan,
    in_paragraph: &mut Option<(Vec<InlineSpan>, usize)>,
    in_heading: &mut Option<ActiveHeading>,
    in_item: &mut Option<ActiveItem>,
    in_table_cell: &mut Option<(Vec<InlineSpan>, String)>,
    in_footnote: &mut Option<(String, Vec<InlineSpan>, usize)>,
    link_stack: &mut [LinkOrImage],
) {
    if let Some(last_link) = link_stack.last_mut() {
        match last_link {
            LinkOrImage::Link { text, .. } => text.push_str(inline.plain_text()),
            LinkOrImage::Image { alt, .. } => alt.push_str(inline.plain_text()),
        }
        return;
    }

    let text_str = inline.plain_text().to_string();

    if let Some(h) = in_heading.as_mut() {
        h.plain.push_str(&text_str);
        h.inlines.push(inline);
    } else if let Some(item) = in_item.as_mut() {
        item.plain.push_str(&text_str);
        item.inlines.push(inline);
    } else if let Some((inlines, plain)) = in_table_cell.as_mut() {
        plain.push_str(&text_str);
        inlines.push(inline);
    } else if let Some((_, inlines, _)) = in_footnote.as_mut() {
        inlines.push(inline);
    } else if let Some((inlines, _)) = in_paragraph.as_mut() {
        inlines.push(inline);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mermaid_fenced_block() {
        let md = "# Title\n\n```mermaid\nflowchart TD\n    A-->B\n```\n\nParagraph.";
        let doc = parse_markdown(md);
        assert_eq!(doc.blocks.len(), 3);

        match &doc.blocks[0] {
            Block::Heading { level, text, .. } => {
                assert_eq!(*level, 1);
                assert_eq!(text, "Title");
            }
            _ => panic!("Expected Heading"),
        }

        match &doc.blocks[1] {
            Block::MermaidBlock { source, .. } => {
                assert!(source.contains("flowchart TD"));
            }
            _ => panic!("Expected MermaidBlock"),
        }
    }

    #[test]
    fn test_parse_gfm_task_lists() {
        let md = "- [ ] Task 1\n- [x] Task 2\n- Simple item";
        let doc = parse_markdown(md);
        assert_eq!(doc.blocks.len(), 1);

        match &doc.blocks[0] {
            Block::List {
                items, is_ordered, ..
            } => {
                assert!(!is_ordered);
                assert_eq!(items.len(), 3);
                assert_eq!(items[0].task_status, Some(false));
                assert_eq!(items[0].text, "Task 1");
                assert_eq!(items[1].task_status, Some(true));
                assert_eq!(items[1].text, "Task 2");
                assert_eq!(items[2].task_status, None);
                assert_eq!(items[2].text, "Simple item");
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_parse_gfm_tables() {
        let md = "| Col A | Col B |\n| :--- | :---: |\n| Cell 1 | Cell 2 |";
        let doc = parse_markdown(md);
        assert_eq!(doc.blocks.len(), 1);

        match &doc.blocks[0] {
            Block::Table {
                headers,
                rows,
                alignments,
                ..
            } => {
                assert_eq!(headers, &["Col A", "Col B"]);
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0], &["Cell 1", "Cell 2"]);
                assert_eq!(
                    alignments,
                    &[ColumnAlignment::Left, ColumnAlignment::Center]
                );
            }
            _ => panic!("Expected Table"),
        }
    }

    #[test]
    fn test_parse_callout_admonitions() {
        let md = "> [!NOTE]\n> This is an important note.";
        let doc = parse_markdown(md);
        assert_eq!(doc.blocks.len(), 1);

        match &doc.blocks[0] {
            Block::BlockQuote { callout, .. } => {
                assert_eq!(*callout, Some(CalloutKind::Note));
            }
            _ => panic!("Expected BlockQuote with callout"),
        }
    }

    #[test]
    fn test_source_line_mapping() {
        let md = "# Line 1\n\nParagraph at line 3.\n\n```rust\nfn main() {}\n```";
        let doc = parse_markdown(md);
        assert_eq!(doc.blocks.len(), 3);

        assert_eq!(doc.blocks[0].span().start_line, 1);
        assert_eq!(doc.blocks[1].span().start_line, 3);
        assert_eq!(doc.blocks[2].span().start_line, 5);

        assert_eq!(doc.find_block_by_line(1), Some(0));
        assert_eq!(doc.find_block_by_line(3), Some(1));
        assert_eq!(doc.find_block_by_line(5), Some(2));
    }

    #[test]
    fn test_heading_slug_generation_and_auto_ids() {
        assert_eq!(generate_slug("Hello World"), "hello-world");
        assert_eq!(generate_slug("  C++ & Rust 2026!  "), "c-rust-2026");
        assert_eq!(generate_slug("測試 標題 123"), "測試-標題-123");
        assert_eq!(generate_slug("!!!"), "");

        let md = "# Heading One\n\n## Custom Heading {#explicit-id}\n\n### !!!\n";
        let doc = parse_markdown(md);
        assert_eq!(doc.blocks.len(), 3);

        match &doc.blocks[0] {
            Block::Heading { id, text, .. } => {
                assert_eq!(text, "Heading One");
                assert_eq!(id.as_deref(), Some("heading-one"));
            }
            _ => panic!("Expected Heading"),
        }

        match &doc.blocks[1] {
            Block::Heading { id, text, .. } => {
                assert_eq!(text, "Custom Heading");
                assert_eq!(id.as_deref(), Some("explicit-id"));
            }
            _ => panic!("Expected Heading with explicit id"),
        }

        match &doc.blocks[2] {
            Block::Heading { id, text, .. } => {
                assert_eq!(text, "!!!");
                assert_eq!(*id, None);
            }
            _ => panic!("Expected Heading"),
        }
    }
}
