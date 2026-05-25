use super::alphabet::{
    is_delimiter, is_identifier_start, is_integer_character, is_operator_start,
    is_valid_source_character, is_wildcard,
};
use super::automata::{
    recognize_comment, recognize_delimiter, recognize_identifier, recognize_integer,
    recognize_keyword_else, recognize_keyword_false, recognize_keyword_fn,
    recognize_keyword_if, recognize_keyword_match, recognize_keyword_meow,
    recognize_keyword_neko, recognize_keyword_nyan, recognize_keyword_true,
    recognize_operator, recognize_string, recognize_wildcard, StringRecognition,
};
use super::symbol_table::SymbolTable;
use super::token::{LexicalError, Token, TokenKind};

use std::fs;    // Filesystem operations (create_dir, write)
use std::io;    // I/O error type used in return values
use std::path::Path;

/// Final result of the lexical analysis pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexicalAnalysis {
    pub tokens: Vec<Token>,
    pub symbols: SymbolTable,
    pub errors: Vec<LexicalError>,
}

impl LexicalAnalysis {
    // `<P: AsRef<Path>>` is a generic constraint: "any type P that can give us a `&Path`".
    // This includes `&str`, `String`, `PathBuf`, `&Path`, etc.
    // In TS: `function writeOutputs<P extends AsRef<Path>>(dir: P): Result<void, IOError>`
    //
    // Returns `io::Result<()>` — `Ok(())` on success, `Err(io::Error)` on failure.
    pub fn write_outputs<P: AsRef<Path>>(&self, output_directory: P) -> io::Result<()> {
        let output_directory = output_directory.as_ref();
        // Create the output directory (and any parents) if it doesn't exist.
        // `?` operator: if this returns `Err`, exit this function early with the error.
        fs::create_dir_all(output_directory)?;

        // Write each output file. `?` propagates any I/O errors upward.
        fs::write(output_directory.join("tokens.txt"), self.render_tokens())?;
        fs::write(output_directory.join("symbols.txt"), self.symbols.render())?;
        fs::write(output_directory.join("errors.txt"), self.render_errors())?;

        Ok(())
    }

    fn render_tokens(&self) -> String {
        let mut output = String::new();
        for token in &self.tokens {
            output.push_str(token.kind.display_name());
            output.push_str(" -> ");
            output.push_str(&token.lexeme);
            output.push('\n');
        }
        output
    }

    fn render_errors(&self) -> String {
        let mut output = String::new();
        for error in &self.errors {
            output.push_str(&error.format_line());
            output.push('\n');
        }
        output
    }
}

/// The lexer stores the source as `Vec<char>` (not `String` or `&str`) so all
/// DFA-style recognizers can inspect the stream using simple index arithmetic
/// with O(1) access. Since Neko only uses ASCII characters, this is both safe
/// and simpler than navigating UTF-8 byte boundaries.
pub struct Lexer {
    source: Vec<char>,
    index: usize,    // Current position in the character array
    line: usize,
    column: usize,
    tokens: Vec<Token>,
    errors: Vec<LexicalError>,
    symbol_table: SymbolTable,
}

impl Lexer {
    // `pub fn new<S: Into<String>>(source: S) -> Self`
    // Generic over `S` — accepts `String`, `&str`, `Box<str>`, etc.
    // `Into<String>` means "any type that can be converted to a `String`".
    // Call `.into()` to perform the conversion.
    pub fn new<S: Into<String>>(source: S) -> Self {
        // Normalize line endings: Windows (\r\n) → \n, old Mac (\r) → \n.
        // This ensures the lexer only deals with \n for line tracking.
        let normalized_source = source.into().replace("\r\n", "\n").replace('\r', "\n");

        Self {
            // `.chars().collect()` converts the String into `Vec<char>`.
            // `.chars()` returns an iterator over Unicode scalar values.
            // `.collect()` gathers them into any collection that implements FromIterator.
            source: normalized_source.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
            errors: Vec::new(),
            symbol_table: SymbolTable::new(),
        }
    }

    // `mut self` — takes ownership of the Lexer, consuming it.
    // After calling `lex()`, the Lexer struct is gone — you cannot use it again.
    // This is intentional: once you've lexed, the accumulated data moves into
    // the `LexicalAnalysis` result. The spent lexer disappears.
    // In TS, the object would still exist; in Rust, ownership is transferred.
    pub fn lex(mut self) -> LexicalAnalysis {
        self.scan_tokens();

        // Move ownership of fields from `self` into the new struct.
        // After this, `self` is no longer accessible.
        LexicalAnalysis {
            tokens: self.tokens,
            symbols: self.symbol_table,
            errors: self.errors,
        }
    }

    // ---- Core scanning loop ----

    fn scan_tokens(&mut self) {
        // `while let Some(x) = expr` — loop while the pattern matches.
        // Equivalent to: `loop { match self.peek() { Some(x) => { ... }, None => break } }`.
        while let Some(current_character) = self.peek() {
            // Dispatch priority order — checked most-specific first.

            // 1. Reject characters outside the valid source character set.
            if !is_valid_source_character(current_character) {
                self.record_error(format!("Invalid character '{}'", current_character));
                self.advance();
                continue;
            }

            // 2. Skip whitespace entirely (tabs, spaces, newlines).
            if current_character.is_ascii_whitespace() {
                self.consume_whitespace();
                continue;
            }

            // 3. Comments start with `#` and go to end of line.
            if current_character == '#' {
                self.scan_comment();
                continue;
            }

            // 4. String literals start and end with double quotes.
            if current_character == '"' {
                self.scan_string_literal();
                continue;
            }

            // 5. Wildcard pattern (`_`) used in match arms.
            if is_wildcard(current_character) {
                self.scan_wildcard();
                continue;
            }

            // 6. Identifiers or keywords (start with a letter).
            if is_identifier_start(current_character) {
                self.scan_identifier_or_keyword();
                continue;
            }

            // 7. Integer literals (start with a digit).
            if is_integer_character(current_character) {
                self.scan_integer();
                continue;
            }

            // 8. Operators (+, -, *, /, %, =, !, <, >, &, |).
            if is_operator_start(current_character) {
                if self.scan_operator_or_delimiter() {
                    continue;
                }
            }

            // 9. Delimiters ((, ), {, }, ,).
            if is_delimiter(current_character) {
                if self.scan_operator_or_delimiter() {
                    continue;
                }
            }

            // 10. If nothing matched above, it's an unexpected symbol.
            self.record_error(format!("Unexpected symbol '{}'", current_character));
            self.advance();
        }
    }

    // ---- Individual scanner methods ----

    // Handles both keywords AND identifiers.
    // Keywords are tried first (they're reserved words), then fall back to identifier.
    fn scan_identifier_or_keyword(&mut self) {
        let start_index = self.index;
        let line = self.line;
        let column = self.column;

        // Try all 9 keyword patterns first.
        if let Some((kind, length)) = self.try_keyword(start_index) {
            let lexeme = self.collect_lexeme(start_index, length);
            self.advance_by(length);
            self.emit_token(kind, lexeme, line, column);
            return;
        }

        // No keyword matched — try as a regular identifier.
        if let Some(length) = recognize_identifier(&self.source, start_index) {
            let lexeme = self.collect_lexeme(start_index, length);
            self.advance_by(length);
            // Add to the symbol table (first appearance order, deduplicated).
            self.symbol_table.insert(&lexeme);
            self.emit_token(TokenKind::Identifier, lexeme, line, column);
            return;
        }

        self.record_error("Invalid identifier".to_string());
        self.advance();
    }

    fn scan_integer(&mut self) {
        let start_index = self.index;
        let line = self.line;
        let column = self.column;

        if let Some(length) = recognize_integer(&self.source, start_index) {
            let lexeme = self.collect_lexeme(start_index, length);
            self.advance_by(length);
            self.emit_token(TokenKind::Integer, lexeme, line, column);
        } else {
            self.record_error("Invalid integer".to_string());
            self.advance();
        }
    }

    fn scan_string_literal(&mut self) {
        let start_index = self.index;
        let line = self.line;
        let column = self.column;

        // `match` on the three-outcome enum returned by `recognize_string`.
        // Rust forces you to handle all variants — no default fallthrough.
        match recognize_string(&self.source, start_index) {
            StringRecognition::Matched(length) => {
                let lexeme = self.collect_lexeme(start_index, length);
                self.advance_by(length);
                self.emit_token(TokenKind::StringLiteral, lexeme, line, column);
            }
            StringRecognition::Unterminated => {
                self.record_error("Unterminated string literal".to_string());
                self.advance();
                // Skip past bad content to prevent cascading errors.
                self.recover_after_string_error();
            }
            StringRecognition::InvalidCharacter { character, offset } => {
                self.record_error_at(
                    line,
                    column + offset,
                    format!("Invalid character '{}' inside string literal", character),
                );
                self.advance();
                self.recover_after_string_error();
            }
        }
    }

    fn scan_comment(&mut self) {
        let start_index = self.index;
        let line = self.line;
        let column = self.column;

        if let Some(length) = recognize_comment(&self.source, start_index) {
            let lexeme = self.collect_lexeme(start_index, length);

            // Validate each character inside the comment.
            for (offset, character) in lexeme.chars().enumerate() {
                if !is_valid_source_character(character) {
                    self.record_error_at(
                        line,
                        column + offset,
                        format!("Invalid character '{}' inside comment", character),
                    );
                }
            }

            self.advance_by(length);
            self.emit_token(TokenKind::Comment, lexeme, line, column);
        }
    }

    fn scan_wildcard(&mut self) {
        let start_index = self.index;
        let line = self.line;
        let column = self.column;

        if let Some(length) = recognize_wildcard(&self.source, start_index) {
            let lexeme = self.collect_lexeme(start_index, length);
            self.advance_by(length);
            self.emit_token(TokenKind::Wildcard, lexeme, line, column);
        } else {
            self.record_error("Invalid wildcard token".to_string());
            self.advance();
        }
    }

    // Handles both operators and delimiters.
    // Returns `true` if a token was matched and emitted.
    fn scan_operator_or_delimiter(&mut self) -> bool {
        let start_index = self.index;
        let line = self.line;
        let column = self.column;

        // Try operators first (they may be multi-character, e.g. "==", "=>").
        if let Some((kind, length)) = recognize_operator(&self.source, start_index) {
            let lexeme = self.collect_lexeme(start_index, length);
            self.advance_by(length);
            self.emit_token(kind, lexeme, line, column);
            return true;
        }

        // Then try delimiters (single character: (, ), {, }, ,).
        if let Some((kind, length)) = recognize_delimiter(&self.source, start_index) {
            let lexeme = self.collect_lexeme(start_index, length);
            self.advance_by(length);
            self.emit_token(kind, lexeme, line, column);
            return true;
        }

        false  // Neither matched
    }

    // Checks all 9 keyword recognizers in priority order.
    // Returns the matched keyword kind and its length, or None.
    fn try_keyword(&self, start_index: usize) -> Option<(TokenKind, usize)> {
        if let Some(length) = recognize_keyword_neko(&self.source, start_index) {
            return Some((TokenKind::KeywordNeko, length));
        }
        if let Some(length) = recognize_keyword_nyan(&self.source, start_index) {
            return Some((TokenKind::KeywordNyan, length));
        }
        if let Some(length) = recognize_keyword_fn(&self.source, start_index) {
            return Some((TokenKind::KeywordFn, length));
        }
        if let Some(length) = recognize_keyword_if(&self.source, start_index) {
            return Some((TokenKind::KeywordIf, length));
        }
        if let Some(length) = recognize_keyword_else(&self.source, start_index) {
            return Some((TokenKind::KeywordElse, length));
        }
        if let Some(length) = recognize_keyword_match(&self.source, start_index) {
            return Some((TokenKind::KeywordMatch, length));
        }
        if let Some(length) = recognize_keyword_meow(&self.source, start_index) {
            return Some((TokenKind::KeywordMeow, length));
        }
        if let Some(length) = recognize_keyword_true(&self.source, start_index) {
            return Some((TokenKind::KeywordTrue, length));
        }
        if let Some(length) = recognize_keyword_false(&self.source, start_index) {
            return Some((TokenKind::KeywordFalse, length));
        }
        None
    }

    // Error recovery: after a string literal error, skip forward until
    // we find the closing quote or a newline. This prevents a single bad
    // character from producing many confusing follow-up errors.
    fn recover_after_string_error(&mut self) {
        while let Some(next_character) = self.peek() {
            if next_character == '"' {
                self.advance();  // Consume the closing quote
                break;
            }
            if next_character == '\n' {
                break;  // Don't consume the newline (scan_tokens handles it)
            }
            self.advance();
        }
    }

    // ---- Helper methods ----

    // Consume all consecutive whitespace characters.
    fn consume_whitespace(&mut self) {
        // `while matches!(expr, pattern if condition)` — loop while the
        // pattern matches with an optional guard condition.
        while matches!(self.peek(), Some(next_character) if next_character.is_ascii_whitespace()) {
            self.advance();
        }
    }

    fn emit_token(&mut self, kind: TokenKind, lexeme: String, line: usize, column: usize) {
        self.tokens.push(Token::new(kind, lexeme, line, column));
    }

    fn record_error(&mut self, message: String) {
        self.errors
            .push(LexicalError::new(self.line, self.column, message));
    }

    // Record an error at a specific position (not necessarily the current one).
    fn record_error_at(&mut self, line: usize, column: usize, message: String) {
        self.errors.push(LexicalError::new(line, column, message));
    }

    // Extract a substring from the source as an owned String.
    // `self.source[start_index..start_index + length]` — range indexing on Vec.
    // In TS: `source.slice(startIndex, startIndex + length).join('')`
    fn collect_lexeme(&self, start_index: usize, length: usize) -> String {
        self.source[start_index..start_index + length].iter().collect()
    }

    // Look at the current character without consuming it.
    // Returns `None` at end of file.
    // `Option<char>` = `char | null` in TS.
    fn peek(&self) -> Option<char> {
        // `.get()` is safe indexing — returns `None` instead of panicking
        // on out-of-bounds. Unlike direct array indexing `self.source[i]`
        // which panics on OOB.
        //
        // `.copied()` converts `Option<&char>` → `Option<char>` — it copies
        // the char value out of the reference (cheap, since char is small).
        self.source.get(self.index).copied()
    }

    // Consume one character and advance the position.
    // Returns the consumed character, or `None` if at end of file.
    // Tracks line and column numbers for error reporting.
    fn advance(&mut self) -> Option<char> {
        // `?` operator: if `peek()` returned `None`, return `None` early.
        let character = self.peek()?;
        self.index += 1;

        if character == '\n' {
            self.line += 1;
            self.column = 1;  // Reset column on new line
        } else {
            self.column += 1;
        }

        Some(character)
    }

    // Advance by N characters, calling `advance()` for each one.
    // This ensures line/column tracking is correct for multi-character tokens.
    fn advance_by(&mut self, count: usize) {
        for _ in 0..count {
            self.advance();
        }
    }
}
