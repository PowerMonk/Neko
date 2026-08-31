// ============================================================================
// SyntaxError
//
// Mirrors the lexer's `LexicalError` shape: a position + a message.
// Two errors with the same line/column/message are not deduplicated —
// the parser reports every problem it finds (collect-don't-bail).
// ============================================================================

/// A single syntactic error with source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl SyntaxError {
    pub fn new(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            column,
            message: message.into(),
        }
    }

    /// `line:col -> message` format, matching the lexer's error file.
    pub fn format_line(&self) -> String {
        format!("{}:{} -> {}", self.line, self.column, self.message)
    }
}
