/// Token categories for the Neko lexer.
///
/// The display names intentionally stay simple and uppercase so the exported
/// `tokens.txt` file is easy to read during compiler exercises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    KeywordNeko,
    KeywordNyan,
    KeywordFn,
    KeywordIf,
    KeywordElse,
    KeywordMatch,
    KeywordMeow,
    KeywordTrue,
    KeywordFalse,
    Identifier,
    Integer,
    StringLiteral,
    Comment,
    Wildcard,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    EqualEqual,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    AndAnd,
    OrOr,
    Bang,
    FatArrow,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
}

impl TokenKind {
    /// Human-readable name used in exported token files.
    pub fn display_name(self) -> &'static str {
        match self {
            TokenKind::KeywordNeko => "NEKO",
            TokenKind::KeywordNyan => "NYAN",
            TokenKind::KeywordFn => "FN",
            TokenKind::KeywordIf => "IF",
            TokenKind::KeywordElse => "ELSE",
            TokenKind::KeywordMatch => "MATCH",
            TokenKind::KeywordMeow => "MEOW",
            TokenKind::KeywordTrue => "TRUE",
            TokenKind::KeywordFalse => "FALSE",
            TokenKind::Identifier => "IDENTIFIER",
            TokenKind::Integer => "INTEGER",
            TokenKind::StringLiteral => "STRING",
            TokenKind::Comment => "COMMENT",
            TokenKind::Wildcard => "WILDCARD",
            TokenKind::Plus => "PLUS",
            TokenKind::Minus => "MINUS",
            TokenKind::Star => "STAR",
            TokenKind::Slash => "SLASH",
            TokenKind::Percent => "PERCENT",
            TokenKind::Assign => "ASSIGN",
            TokenKind::EqualEqual => "EQUAL_EQUAL",
            TokenKind::NotEqual => "NOT_EQUAL",
            TokenKind::Less => "LESS",
            TokenKind::Greater => "GREATER",
            TokenKind::LessEqual => "LESS_EQUAL",
            TokenKind::GreaterEqual => "GREATER_EQUAL",
            TokenKind::AndAnd => "AND_AND",
            TokenKind::OrOr => "OR_OR",
            TokenKind::Bang => "BANG",
            TokenKind::FatArrow => "FAT_ARROW",
            TokenKind::LeftParen => "LEFT_PAREN",
            TokenKind::RightParen => "RIGHT_PAREN",
            TokenKind::LeftBrace => "LEFT_BRACE",
            TokenKind::RightBrace => "RIGHT_BRACE",
            TokenKind::Comma => "COMMA",
        }
    }
}

/// A single token produced by the lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            kind,
            lexeme: lexeme.into(),
            line,
            column,
        }
    }
}

/// A lexical error with source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexicalError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl LexicalError {
    pub fn new(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            column,
            message: message.into(),
        }
    }

    pub fn format_line(&self) -> String {
        format!("{}:{} -> {}", self.line, self.column, self.message)
    }
}
