use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 1-indexed source line and 0-indexed byte offset span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl SourceSpan {
    pub fn new(start_line: usize, end_line: usize, start_byte: usize, end_byte: usize) -> Self {
        Self {
            start_line,
            end_line: end_line.max(start_line),
            start_byte,
            end_byte: end_byte.max(start_byte),
        }
    }

    pub fn line(line: usize) -> Self {
        Self {
            start_line: line,
            end_line: line,
            start_byte: 0,
            end_byte: 0,
        }
    }
}

/// GFM table column alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ColumnAlignment {
    #[default]
    None,
    Left,
    Center,
    Right,
}

/// GitHub-style blockquote callout kind (admonition).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalloutKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

/// Inline text styling attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
}

/// Rich inline element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InlineSpan {
    Text(String),
    Bold(String),
    Italic(String),
    Strikethrough(String),
    Code(String),
    StyledText {
        text: String,
        style: InlineStyle,
    },
    Link {
        text: String,
        url: String,
        title: Option<String>,
    },
    Image {
        alt: String,
        url: String,
        title: Option<String>,
        resolved_path: Option<PathBuf>,
    },
    FootnoteRef(String),
    SoftBreak,
    HardBreak,
}

impl InlineSpan {
    pub fn plain_text(&self) -> &str {
        match self {
            InlineSpan::Text(s)
            | InlineSpan::Bold(s)
            | InlineSpan::Italic(s)
            | InlineSpan::Strikethrough(s)
            | InlineSpan::Code(s)
            | InlineSpan::StyledText { text: s, .. } => s,
            InlineSpan::Link { text, .. } => text,
            InlineSpan::Image { alt, .. } => alt,
            InlineSpan::FootnoteRef(s) => s,
            InlineSpan::SoftBreak => " ",
            InlineSpan::HardBreak => "\n",
        }
    }
}

/// Single item within an ordered, unordered, or task list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItem {
    /// Plain-text content of the item (for search and fast outline)
    pub text: String,
    /// Rich inlines (supports bold, links, code inside list items)
    pub inlines: Vec<InlineSpan>,
    /// Task checkbox state: None for normal bullet/numbered item,
    /// Some(false) for `- [ ]`, Some(true) for `- [x]`.
    pub task_status: Option<bool>,
    /// Source span of this specific item
    pub span: SourceSpan,
}

/// Top-level block elements in the Document Model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Block {
    Heading {
        level: u8,
        text: String,
        inlines: Vec<InlineSpan>,
        id: Option<String>,
        classes: Vec<String>,
        line: usize,
        span: SourceSpan,
    },
    Paragraph {
        inlines: Vec<InlineSpan>,
        line: usize,
        span: SourceSpan,
    },
    BlockQuote {
        lines: Vec<String>,
        inlines: Vec<InlineSpan>,
        callout: Option<CalloutKind>,
        line: usize,
        span: SourceSpan,
    },
    CodeBlock {
        language: String,
        code: String,
        line: usize,
        span: SourceSpan,
    },
    MermaidBlock {
        source: String,
        line: usize,
        span: SourceSpan,
    },
    List {
        items: Vec<ListItem>,
        is_ordered: bool,
        start_number: Option<u64>,
        line: usize,
        span: SourceSpan,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        alignments: Vec<ColumnAlignment>,
        header_inlines: Vec<Vec<InlineSpan>>,
        row_inlines: Vec<Vec<Vec<InlineSpan>>>,
        line: usize,
        span: SourceSpan,
    },
    ThematicBreak {
        line: usize,
        span: SourceSpan,
    },
    FootnoteDefinition {
        label: String,
        inlines: Vec<InlineSpan>,
        line: usize,
        span: SourceSpan,
    },
}

impl Block {
    pub fn line(&self) -> usize {
        match self {
            Block::Heading { line, .. }
            | Block::Paragraph { line, .. }
            | Block::BlockQuote { line, .. }
            | Block::CodeBlock { line, .. }
            | Block::MermaidBlock { line, .. }
            | Block::List { line, .. }
            | Block::Table { line, .. }
            | Block::ThematicBreak { line, .. }
            | Block::FootnoteDefinition { line, .. } => *line,
        }
    }

    pub fn span(&self) -> SourceSpan {
        match self {
            Block::Heading { span, .. }
            | Block::Paragraph { span, .. }
            | Block::BlockQuote { span, .. }
            | Block::CodeBlock { span, .. }
            | Block::MermaidBlock { span, .. }
            | Block::List { span, .. }
            | Block::Table { span, .. }
            | Block::ThematicBreak { span, .. }
            | Block::FootnoteDefinition { span, .. } => *span,
        }
    }
}

/// Headless document model containing all blocks, footnotes, and line mappings.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DocModel {
    pub blocks: Vec<Block>,
    pub footnotes: HashMap<String, Vec<InlineSpan>>,
    pub total_lines: usize,
}

impl DocModel {
    pub fn new(blocks: Vec<Block>, total_lines: usize) -> Self {
        Self {
            blocks,
            footnotes: HashMap::new(),
            total_lines,
        }
    }

    /// Finds index of block enclosing or closest to 1-indexed source line.
    pub fn find_block_by_line(&self, line: usize) -> Option<usize> {
        if self.blocks.is_empty() {
            return None;
        }
        let idx = match self
            .blocks
            .binary_search_by(|b| b.span().start_line.cmp(&line))
        {
            Ok(found) => found,
            Err(insert_idx) => {
                if insert_idx == 0 {
                    0
                } else {
                    let prev = insert_idx - 1;
                    if line <= self.blocks[prev].span().end_line {
                        prev
                    } else if insert_idx < self.blocks.len() {
                        let prev_dist = line.saturating_sub(self.blocks[prev].span().end_line);
                        let next_dist = self.blocks[insert_idx]
                            .span()
                            .start_line
                            .saturating_sub(line);
                        if next_dist < prev_dist {
                            insert_idx
                        } else {
                            prev
                        }
                    } else {
                        prev
                    }
                }
            }
        };
        Some(idx.min(self.blocks.len() - 1))
    }

    /// Extracts outline headings for table-of-contents navigation.
    pub fn outline(&self) -> Vec<(u8, String, usize)> {
        self.blocks
            .iter()
            .filter_map(|b| match b {
                Block::Heading {
                    level, text, line, ..
                } => Some((*level, text.clone(), *line)),
                _ => None,
            })
            .collect()
    }
}
