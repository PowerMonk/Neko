// Character-level classifiers for the Neko language.
// Each function answers one true/false question about a single character.
// These are pure functions — no state, no side effects — used as building
// blocks by the DFA recognizers in automata.rs.
//
// In TS: each would be a standalone function like:
//   function isIdentifierStart(char: string): boolean

pub fn is_valid_source_character(character: char) -> bool {
    // `is_ascii_graphic()` = any visible ASCII (letters, digits, punctuation, symbols)
    // `is_ascii_whitespace()` = space, tab, newline, carriage return, form feed
    character.is_ascii_graphic() || character.is_ascii_whitespace()
}

pub fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic()
}

// `is_identifier_continue` — characters allowed after the first character.
// Identifiers can contain letters, digits, or underscores (like TS variable names).
pub fn is_identifier_continue(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

pub fn is_integer_character(character: char) -> bool {
    character.is_ascii_digit()
}

// Characters allowed inside string literals (alphanumeric, underscore, space).
pub fn is_string_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_' || character == ' '
}

pub fn is_wildcard(character: char) -> bool {
    character == '_'
}

// `matches!` is a Rust macro that checks a value against a pattern.
// It expands to something like:
//   match character {
//       '+' | '-' | '*' | ... => true,
//       _ => false,
//   }
// In TS: `['+', '-', '*', ...].includes(char)`
pub fn is_operator_start(character: char) -> bool {
    matches!(
        character,
        '+' | '-' | '*' | '/' | '%' | '=' | '!' | '<' | '>' | '&' | '|'
    )
}

pub fn is_delimiter(character: char) -> bool {
    matches!(character, '(' | ')' | '{' | '}' | ',' | ';')
}
