// ============================================================================
// TokenStream — read-only cursor over the lexer's token output.
//
// Why a wrapper?
// -------------
// The lexer's output is `Vec<Token>` (an owned list). The parser needs to
// repeatedly ask "what's next?", "consume the next one", and "does the next
// one match this kind?". Wrapping the Vec in a tiny struct keeps that
// logic in one place so every parse function can share the same
// conventions and so tests can construct parsers from arbitrary token
// sequences without touching the file system.
//
// SAFETY NOTE: this is NOT a borrow. `tokens` is owned by the stream. The
// Parser owns the stream. Both are consumed (`mut self`) at the end of
// parsing. After that only the resulting ParseResult survives.
// ============================================================================

use crate::lexer::{Token, TokenKind};

pub struct TokenStream {
    tokens: Vec<Token>,
    /// Index of the next token to read. `tokens.len()` means EOF.
    index: usize,
}

impl TokenStream {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
    }

    /// Look at the current token without consuming it.
    /// Returns `None` at EOF.
    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    /// Look N tokens ahead without consuming them. `peek_at(0) == peek()`.
    #[allow(dead_code)]
    pub fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.index + offset)
    }

    /// Consume and return the current token.
    pub fn advance(&mut self) -> Option<Token> {
        if self.index < self.tokens.len() {
            let t = self.tokens[self.index].clone();
            self.index += 1;
            Some(t)
        } else {
            None
        }
    }

    /// Are we at end of input?
    pub fn is_eof(&self) -> bool {
        self.index >= self.tokens.len()
    }

    /// Convenience: peek the current token's kind.
    pub fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|t| t.kind)
    }

    /// If the current token's kind matches `kind`, consume it and return
    /// it. Otherwise return `None` without consuming.
    pub fn consume(&mut self, kind: TokenKind) -> Option<Token> {
        match self.peek_kind() {
            Some(k) if k == kind => self.advance(),
            _ => None,
        }
    }

    /// Borrow the entire token list (used by the pretty-printer).
    #[allow(dead_code)]
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// Current cursor position (used for error reporting / debug).
    #[allow(dead_code)]
    pub fn position(&self) -> usize {
        self.index
    }
}
