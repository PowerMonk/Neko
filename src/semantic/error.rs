// ============================================================================
// SemanticError
//
// One error per detected semantic violation. The message format mirrors
// the lexer's and parser's error format so `output/semantic_errors.txt`
// has the same look as `output/errors.txt` and `output/parse_errors.txt`.
// ============================================================================

/// A semantic error with source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl SemanticError {
    pub fn new(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            column,
            message: message.into(),
        }
    }

    /// `line:col -> message` format used by all three error files in `output/`.
    pub fn format_line(&self) -> String {
        format!("{}:{} -> {}", self.line, self.column, self.message)
    }
}
