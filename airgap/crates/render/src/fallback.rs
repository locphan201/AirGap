use crate::mermaid::MermaidError;

/// Structured diagnostic model for rendering graceful fallback cards when a Mermaid diagram fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramErrorFallback {
    pub error_title: String,
    pub error_message: String,
    pub error_line: usize,
    pub source_lines: Vec<(usize, String, bool)>, // (line_number, content, is_error_line)
}

impl DiagramErrorFallback {
    pub fn from_error(source: &str, error: &MermaidError) -> Self {
        let (error_title, error_message, error_line) = match error {
            MermaidError::EmptySource => (
                "Empty Mermaid Diagram".to_string(),
                "The Mermaid code block contains no diagram syntax.".to_string(),
                1,
            ),
            MermaidError::SyntaxError { line, message } => (
                format!("Mermaid Syntax Error (Line {})", line),
                message.clone(),
                *line,
            ),
            MermaidError::RenderError(msg) => ("Mermaid Render Error".to_string(), msg.clone(), 1),
            MermaidError::Panic(msg) => (
                "Mermaid Parser Panic (Recovered)".to_string(),
                format!("Parser safely recovered from panic: {}", msg),
                1,
            ),
        };

        let source_lines = source
            .lines()
            .enumerate()
            .map(|(idx, line)| {
                let line_num = idx + 1;
                let is_err = line_num == error_line;
                (line_num, line.to_string(), is_err)
            })
            .collect();

        Self {
            error_title,
            error_message,
            error_line,
            source_lines,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_diagnostics() {
        let source = "flowchart TD\n  A --> [broken";
        let err = MermaidError::SyntaxError {
            line: 2,
            message: "Unexpected token".to_string(),
        };
        let fallback = DiagramErrorFallback::from_error(source, &err);
        assert_eq!(fallback.error_line, 2);
        assert_eq!(fallback.source_lines.len(), 2);
        assert!(fallback.source_lines[1].2); // second line is error line
    }
}
