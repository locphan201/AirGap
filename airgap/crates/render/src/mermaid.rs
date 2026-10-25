use mermaid_rs_renderer::{render_with_options, LayoutConfig, RenderOptions, Theme};
use std::panic::{catch_unwind, AssertUnwindSafe};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum MermaidError {
    #[error("Empty diagram source")]
    EmptySource,

    #[error("Mermaid syntax error on line {line}: {message}")]
    SyntaxError { line: usize, message: String },

    #[error("Mermaid render error: {0}")]
    RenderError(String),

    #[error("Mermaid parser panicked: {0}")]
    Panic(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagramType {
    Flowchart,
    Sequence,
    Class,
    State,
    Er,
    Pie,
    Gantt,
    Unknown,
}

impl DiagramType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagramType::Flowchart => "Flowchart / Graph",
            DiagramType::Sequence => "Sequence Diagram",
            DiagramType::Class => "Class Diagram",
            DiagramType::State => "State Diagram",
            DiagramType::Er => "Entity-Relationship (ER)",
            DiagramType::Pie => "Pie Chart",
            DiagramType::Gantt => "Gantt Chart",
            DiagramType::Unknown => "Unknown / Other",
        }
    }
}

/// Detects the diagram type from the source text.
pub fn detect_diagram_type(source: &str) -> DiagramType {
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        let lower = trimmed.to_lowercase();
        if lower.starts_with("flowchart") || lower.starts_with("graph") {
            return DiagramType::Flowchart;
        } else if lower.starts_with("sequencediagram") {
            return DiagramType::Sequence;
        } else if lower.starts_with("classdiagram") {
            return DiagramType::Class;
        } else if lower.starts_with("statediagram") {
            return DiagramType::State;
        } else if lower.starts_with("erdiagram") {
            return DiagramType::Er;
        } else if lower.starts_with("pie") {
            return DiagramType::Pie;
        } else if lower.starts_with("gantt") {
            return DiagramType::Gantt;
        } else {
            return DiagramType::Unknown;
        }
    }
    DiagramType::Unknown
}

/// Renders a Mermaid diagram code block to an SVG string.
/// Wrapped in `catch_unwind` to guarantee that invalid or pathological inputs never crash the application.
pub fn render_mermaid_to_svg(source: &str, is_dark: bool) -> Result<String, MermaidError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(MermaidError::EmptySource);
    }

    let source_owned = trimmed.to_string();

    let render_result = catch_unwind(AssertUnwindSafe(|| {
        let theme = if is_dark {
            Theme::dark()
        } else {
            Theme::modern()
        };

        let options = RenderOptions {
            theme,
            layout: LayoutConfig::default(),
        };

        render_with_options(&source_owned, options)
    }));

    match render_result {
        Ok(Ok(svg)) => Ok(svg),
        Ok(Err(err)) => {
            let err_str = err.to_string();
            // Attempt to parse line number from error message
            let line = extract_error_line(&err_str).unwrap_or(1);
            Err(MermaidError::SyntaxError {
                line,
                message: err_str,
            })
        }
        Err(panic_err) => {
            let msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_err.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic during Mermaid rendering".to_string()
            };
            Err(MermaidError::Panic(msg))
        }
    }
}

fn extract_error_line(err_msg: &str) -> Option<usize> {
    // Look for patterns like "line 4" or "Line 4"
    let lower = err_msg.to_lowercase();
    if let Some(pos) = lower.find("line ") {
        let rest = &lower[pos + 5..];
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(line_num) = num_str.parse::<usize>() {
            return Some(line_num);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flowchart_rendering() {
        let source = "flowchart TD\n    A[Start] --> B{Is it working?}\n    B -->|Yes| C[Great!]\n    B -->|No| D[Debug]";
        let svg = render_mermaid_to_svg(source, false).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Start"));
    }

    #[test]
    fn test_sequence_rendering() {
        let source = "sequenceDiagram\n    Alice->>Bob: Hello Bob\n    Bob-->>Alice: Hi Alice";
        let svg = render_mermaid_to_svg(source, true).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Alice"));
    }

    #[test]
    fn test_class_rendering() {
        let source = "classDiagram\n    class BankAccount {\n        +String owner\n        +deposit(amount)\n    }";
        let svg = render_mermaid_to_svg(source, false).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("BankAccount"));
    }

    #[test]
    fn test_state_rendering() {
        let source = "stateDiagram-v2\n    [*] --> Still\n    Still --> [*]";
        let svg = render_mermaid_to_svg(source, true).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Still"));
    }

    #[test]
    fn test_er_rendering() {
        let source = "erDiagram\n    CUSTOMER ||--o{ ORDER : places";
        let svg = render_mermaid_to_svg(source, false).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("CUSTOMER"));
    }

    #[test]
    fn test_pie_rendering() {
        let source = "pie title Pets\n    \"Dogs\" : 386\n    \"Cats\" : 85";
        let svg = render_mermaid_to_svg(source, true).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Dogs"));
    }

    #[test]
    fn test_gantt_rendering() {
        let source = "gantt\n    title A Gantt Diagram\n    dateFormat YYYY-MM-DD\n    section Section\n    A task :a1, 2026-01-01, 30d";
        let svg = render_mermaid_to_svg(source, false).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("A task"));
    }

    #[test]
    fn test_malformed_diagram_error_handling() {
        let broken = "unknown_diagram_type\n    A ---> ???";
        let res = render_mermaid_to_svg(broken, false);
        assert!(res.is_err());
        match res.unwrap_err() {
            MermaidError::SyntaxError { message, .. } => {
                assert!(!message.is_empty());
            }
            _ => panic!("Expected SyntaxError"),
        }
    }
}
