// TokenKind — every possible token category in the Neko language.
//
// `#[derive(...)]` is a macro that auto-implements common traits for this enum:
//   - `Debug`   → allows `{:?}` formatting (like console.log)
//   - `Clone`   → allows `.clone()` to make a deep copy
//   - `Copy`    → allows implicit bitwise copy (cheap — no heap data, no destructor)
//   - `PartialEq` → allows `==` and `!=` comparisons
//   - `Eq`      → full equality contract (no floating-point NaN)
//
// Rust enums are "sum types" — each variant is a distinct case, and variants
// can carry different data payloads. This enum has no payloads (unit variants).
// In TS: `type TokenKind = "KeywordNeko" | "KeywordNyan" | "KeywordFn" | ...`
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
    // `&'static str` — a string literal reference that lives for the entire program.
    // Static strings are embedded in the compiled binary and never freed.
    // Safe to pass around as a read-only reference.
    pub fn display_name(self) -> &'static str {
        match self {
            // `match` in Rust is like a super-powered `switch` — the compiler
            // checks exhaustively that EVERY variant is covered.
            // Missing a variant = compile error (unlike TS where you get undefined).
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
    pub lexeme: String,  // The raw text matched (e.g. "banana", "90", "=>")
    pub line: usize,
    pub column: usize,
}

impl Token {
    // `impl Into<String>` — accept any type that can be converted into `String`.
    // This includes `&str`, `String`, `Box<str>`, `Cow<'_, str>`, etc.
    // Call `.into()` to perform the conversion.
    // In TS: this is like `function new(kind, lexeme: string | Stringable)`
    pub fn new(kind: TokenKind, lexeme: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            kind,
            // `.into()` converts the generic input into an owned `String`.
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

    // `format!()` is like `println!()` but returns a `String` instead of printing.
    // In TS: `` `${this.line}:${this.column} -> ${this.message}` ``
    pub fn format_line(&self) -> String {
        format!("{}:{} -> {}", self.line, self.column, self.message)
    }
}
