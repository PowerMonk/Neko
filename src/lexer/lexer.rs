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

// std::fs - provides filesystem operations (reading/writing files, creating directories)
use std::fs;
// std::io - provides input/output traits and utilities for handling I/O operations
use std::io;
use std::path::Path;

/// Final result of the lexical analysis pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexicalAnalysis {
    pub tokens: Vec<Token>,
    pub symbols: SymbolTable,
    pub errors: Vec<LexicalError>,
}

impl LexicalAnalysis {
    pub fn write_outputs<P: AsRef<Path>>(&self, output_directory: P) -> io::Result<()> {
        let output_directory = output_directory.as_ref();
        fs::create_dir_all(output_directory)?;

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

/// The lexer keeps the source as characters so all DFA-style recognizers can
/// inspect the stream with simple index arithmetic.
pub struct Lexer {
    source: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
    errors: Vec<LexicalError>,
    symbol_table: SymbolTable,
}

impl Lexer {
    pub fn new<S: Into<String>>(source: S) -> Self {
        let normalized_source = source.into().replace("\r\n", "\n").replace('\r', "\n");

        Self {
            source: normalized_source.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
            errors: Vec::new(),
            symbol_table: SymbolTable::new(),
        }
    }

    pub fn lex(mut self) -> LexicalAnalysis {
        self.scan_tokens();

        LexicalAnalysis {
            tokens: self.tokens,
            symbols: self.symbol_table,
            errors: self.errors,
        }
    }

    fn scan_tokens(&mut self) {
        while let Some(current_character) = self.peek() {
            if !is_valid_source_character(current_character) {
                self.record_error(format!("Invalid character '{}'", current_character));
                self.advance();
                continue;
            }

            if current_character.is_ascii_whitespace() {
                self.consume_whitespace();
                continue;
            }

            if current_character == '#' {
                self.scan_comment();
                continue;
            }

            if current_character == '"' {
                self.scan_string_literal();
                continue;
            }

            if is_wildcard(current_character) {
                self.scan_wildcard();
                continue;
            }

            if is_identifier_start(current_character) {
                self.scan_identifier_or_keyword();
                continue;
            }

            if is_integer_character(current_character) {
                self.scan_integer();
                continue;
            }

            if is_operator_start(current_character) {
                if self.scan_operator_or_delimiter() {
                    continue;
                }
            }

            if is_delimiter(current_character) {
                if self.scan_operator_or_delimiter() {
                    continue;
                }
            }

            self.record_error(format!("Unexpected symbol '{}'", current_character));
            self.advance();
        }
    }

    fn scan_identifier_or_keyword(&mut self) {
        let start_index = self.index;
        let line = self.line;
        let column = self.column;

        if let Some((kind, length)) = self.try_keyword(start_index) {
            let lexeme = self.collect_lexeme(start_index, length);
            self.advance_by(length);
            self.emit_token(kind, lexeme, line, column);
            return;
        }

        if let Some(length) = recognize_identifier(&self.source, start_index) {
            let lexeme = self.collect_lexeme(start_index, length);
            self.advance_by(length);
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

        match recognize_string(&self.source, start_index) {
            StringRecognition::Matched(length) => {
                let lexeme = self.collect_lexeme(start_index, length);
                self.advance_by(length);
                self.emit_token(TokenKind::StringLiteral, lexeme, line, column);
            }
            StringRecognition::Unterminated => {
                self.record_error("Unterminated string literal".to_string());
                self.advance();
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

    fn scan_operator_or_delimiter(&mut self) -> bool {
        let start_index = self.index;
        let line = self.line;
        let column = self.column;

        if let Some((kind, length)) = recognize_operator(&self.source, start_index) {
            let lexeme = self.collect_lexeme(start_index, length);
            self.advance_by(length);
            self.emit_token(kind, lexeme, line, column);
            return true;
        }

        if let Some((kind, length)) = recognize_delimiter(&self.source, start_index) {
            let lexeme = self.collect_lexeme(start_index, length);
            self.advance_by(length);
            self.emit_token(kind, lexeme, line, column);
            return true;
        }

        false
    }

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

    fn recover_after_string_error(&mut self) {
        while let Some(next_character) = self.peek() {
            if next_character == '"' {
                self.advance();
                break;
            }

            if next_character == '\n' {
                break;
            }

            self.advance();
        }
    }

    fn consume_whitespace(&mut self) {
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

    fn record_error_at(&mut self, line: usize, column: usize, message: String) {
        self.errors.push(LexicalError::new(line, column, message));
    }

    fn collect_lexeme(&self, start_index: usize, length: usize) -> String {
        self.source[start_index..start_index + length].iter().collect()
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.index).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.index += 1;

        if character == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(character)
    }

    fn advance_by(&mut self, count: usize) {
        for _ in 0..count {
            self.advance();
        }
    }
}
