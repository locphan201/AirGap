use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightedSpan {
    pub text: String,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl HighlightedSpan {
    pub fn new(text: impl Into<String>, r: u8, g: u8, b: u8) -> Self {
        Self {
            text: text.into(),
            r,
            g,
            b,
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeThemePalette {
    pub bg: (u8, u8, u8),
    pub fg: (u8, u8, u8),
}

pub struct SyntaxHighlighter {
    syntax_set: &'static SyntaxSet,
    theme_set: &'static ThemeSet,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines),
            theme_set: THEME_SET.get_or_init(ThemeSet::load_defaults),
        }
    }

    /// Resolves common language aliases to Syntect registered syntax identifiers.
    pub fn resolve_language(&self, lang: &str) -> &'static str {
        match lang.trim().to_lowercase().as_str() {
            "rs" | "rust" => "Rust",
            "py" | "python" => "Python",
            "js" | "javascript" => "JavaScript",
            "ts" | "typescript" => "TypeScript",
            "sh" | "bash" | "zsh" | "shell" => "Bourne Again Shell (bash)",
            "c" => "C",
            "cpp" | "c++" => "C++",
            "cs" | "csharp" => "C#",
            "html" | "htm" => "HTML",
            "xml" | "svg" => "XML",
            "json" => "JSON",
            "yaml" | "yml" => "YAML",
            "toml" => "TOML",
            "md" | "markdown" => "Markdown",
            "sql" => "SQL",
            "go" | "golang" => "Go",
            "java" => "Java",
            _ => "Plain Text",
        }
    }

    pub fn theme_palette(&self, is_dark: bool) -> CodeThemePalette {
        let theme_name = if is_dark {
            "base16-ocean.dark"
        } else {
            "InspiredGitHub"
        };
        let theme = &self.theme_set.themes[theme_name];
        let bg = theme.settings.background.unwrap_or(if is_dark {
            Color {
                r: 24,
                g: 24,
                b: 27,
                a: 255,
            }
        } else {
            Color {
                r: 244,
                g: 244,
                b: 245,
                a: 255,
            }
        });
        let fg = theme.settings.foreground.unwrap_or(if is_dark {
            Color {
                r: 212,
                g: 212,
                b: 216,
                a: 255,
            }
        } else {
            Color {
                r: 39,
                g: 39,
                b: 42,
                a: 255,
            }
        });
        CodeThemePalette {
            bg: (bg.r, bg.g, bg.b),
            fg: (fg.r, fg.g, fg.b),
        }
    }

    /// Highlights code lines with correct newline handling and font style flags.
    pub fn highlight_code(
        &self,
        code: &str,
        language: &str,
        is_dark: bool,
    ) -> Vec<Vec<HighlightedSpan>> {
        let syntax_name = self.resolve_language(language);
        let syntax = self
            .syntax_set
            .find_syntax_by_name(syntax_name)
            .or_else(|| self.syntax_set.find_syntax_by_token(language))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme_name = if is_dark {
            "base16-ocean.dark"
        } else {
            "InspiredGitHub"
        };
        let theme = &self.theme_set.themes[theme_name];
        let mut highlighter = HighlightLines::new(syntax, theme);

        let mut lines = Vec::new();
        for line in LinesWithEndings::from(code) {
            match highlighter.highlight_line(line, self.syntax_set) {
                Ok(spans) => {
                    let line_spans: Vec<HighlightedSpan> = spans
                        .into_iter()
                        .map(|(style, text)| {
                            let font_style = style.font_style;
                            HighlightedSpan {
                                text: text.to_string(),
                                r: style.foreground.r,
                                g: style.foreground.g,
                                b: style.foreground.b,
                                bold: font_style.contains(FontStyle::BOLD),
                                italic: font_style.contains(FontStyle::ITALIC),
                                underline: font_style.contains(FontStyle::UNDERLINE),
                            }
                        })
                        .collect();
                    lines.push(line_spans);
                }
                Err(_) => {
                    lines.push(vec![HighlightedSpan {
                        text: line.to_string(),
                        r: if is_dark { 212 } else { 39 },
                        g: if is_dark { 212 } else { 39 },
                        b: if is_dark { 216 } else { 42 },
                        bold: false,
                        italic: false,
                        underline: false,
                    }]);
                }
            }
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_highlight_rust() {
        let highlighter = SyntaxHighlighter::new();
        let code = "fn main() {\n    let x = 42;\n}";
        let lines = highlighter.highlight_code(code, "rust", true);
        assert_eq!(lines.len(), 3);
        assert!(!lines[0].is_empty());
        assert_eq!(lines[0][0].text, "fn");
    }

    #[test]
    fn test_syntax_highlight_python_light() {
        let highlighter = SyntaxHighlighter::new();
        let code = "def hello():\n    return 'world'\n";
        let lines = highlighter.highlight_code(code, "py", false);
        assert!(lines.len() >= 2);
        assert!(!lines[0].is_empty());
        assert_eq!(lines[0][0].text, "def");
    }

    #[test]
    fn test_theme_palettes() {
        let highlighter = SyntaxHighlighter::new();
        let dark_palette = highlighter.theme_palette(true);
        let light_palette = highlighter.theme_palette(false);
        assert_ne!(dark_palette.bg, light_palette.bg);
    }
}
