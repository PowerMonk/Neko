use super::alphabet::{
    is_identifier_continue, is_identifier_start, is_integer_character, is_string_character,
};
use super::token::TokenKind;

// This enum has three "variants" that carry different data payloads.
// In TS, this would be a discriminated union:
//   type StringRecognition =
//     | { kind: "Matched"; length: number }
//     | { kind: "Unterminated" }
//     | { kind: "InvalidCharacter"; character: string; offset: number }
pub enum StringRecognition {
    Matched(usize),              // Named tuple: just a length value
    Unterminated,                // No data needed — just the fact it happened
    InvalidCharacter {           // Named fields — like an object
        character: char,
        offset: usize,
    },
}

// Ensures a keyword isn't followed by identifier-continuation characters.
// This prevents "neko" from matching inside "nekocat" — the `n` at position 4
// would be `is_identifier_continue(n)` → true, so `keyword_boundary` returns false.
fn keyword_boundary(chars: &[char], end_index: usize) -> bool {
    // `chars.get(end_index)` — safe index lookup, returns `Option<&char>` (never panics).
    // `.copied()` converts `Option<&char>` → `Option<char>` by copying the value.
    // `match` on `Option`:
    //   `Some(next_character)` — there IS a character at this position
    //   `None` — we're at end of file (EOF), which IS a valid keyword boundary
    match chars.get(end_index).copied() {
        Some(next_character) => !is_identifier_continue(next_character),
        None => true,  // EOF = boundary
    }
}

// --- Keyword recognizers ---
// Each is a hand-written DFA (Deterministic Finite Automaton).
// The comment `q0 --n--> q1` means "from state 0, on character 'n', transition to state 1".
// `(accept)` means the final state is reached — return the matched length.

/// q0 --n--> q1 --e--> q2 --k--> q3 --o--> q4 (accept)
pub fn recognize_keyword_neko(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start).copied() != Some('n') {
        return None;
    }
    if chars.get(start + 1).copied() != Some('e') {
        return None;
    }
    if chars.get(start + 2).copied() != Some('k') {
        return None;
    }
    if chars.get(start + 3).copied() != Some('o') {
        return None;
    }
    // After matching all 4 characters, verify it's a word boundary
    // (next char is not alphanumeric or underscore).
    if !keyword_boundary(chars, start + 4) {
        return None;
    }
    Some(4)
}

/// q0 --n--> q1 --y--> q2 --a--> q3 --n--> q4 (accept)
pub fn recognize_keyword_nyan(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start).copied() != Some('n') {
        return None;
    }
    if chars.get(start + 1).copied() != Some('y') {
        return None;
    }
    if chars.get(start + 2).copied() != Some('a') {
        return None;
    }
    if chars.get(start + 3).copied() != Some('n') {
        return None;
    }
    if !keyword_boundary(chars, start + 4) {
        return None;
    }
    Some(4)
}

/// q0 --f--> q1 --n--> q2 (accept)
pub fn recognize_keyword_fn(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start).copied() != Some('f') {
        return None;
    }
    if chars.get(start + 1).copied() != Some('n') {
        return None;
    }
    if !keyword_boundary(chars, start + 2) {
        return None;
    }
    Some(2)
}

/// q0 --i--> q1 --f--> q2 (accept)
pub fn recognize_keyword_if(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start).copied() != Some('i') {
        return None;
    }
    if chars.get(start + 1).copied() != Some('f') {
        return None;
    }
    if !keyword_boundary(chars, start + 2) {
        return None;
    }
    Some(2)
}

/// q0 --e--> q1 --l--> q2 --s--> q3 --e--> q4 (accept)
pub fn recognize_keyword_else(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start).copied() != Some('e') {
        return None;
    }
    if chars.get(start + 1).copied() != Some('l') {
        return None;
    }
    if chars.get(start + 2).copied() != Some('s') {
        return None;
    }
    if chars.get(start + 3).copied() != Some('e') {
        return None;
    }
    if !keyword_boundary(chars, start + 4) {
        return None;
    }
    Some(4)
}

/// q0 --m--> q1 --a--> q2 --t--> q3 --c--> q4 --h--> q5 (accept)
pub fn recognize_keyword_match(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start).copied() != Some('m') {
        return None;
    }
    if chars.get(start + 1).copied() != Some('a') {
        return None;
    }
    if chars.get(start + 2).copied() != Some('t') {
        return None;
    }
    if chars.get(start + 3).copied() != Some('c') {
        return None;
    }
    if chars.get(start + 4).copied() != Some('h') {
        return None;
    }
    if !keyword_boundary(chars, start + 5) {
        return None;
    }
    Some(5)
}

/// q0 --m--> q1 --e--> q2 --o--> q3 --w--> q4 (accept)
pub fn recognize_keyword_meow(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start).copied() != Some('m') {
        return None;
    }
    if chars.get(start + 1).copied() != Some('e') {
        return None;
    }
    if chars.get(start + 2).copied() != Some('o') {
        return None;
    }
    if chars.get(start + 3).copied() != Some('w') {
        return None;
    }
    if !keyword_boundary(chars, start + 4) {
        return None;
    }
    Some(4)
}

/// q0 --t--> q1 --r--> q2 --u--> q3 --e--> q4 (accept)
pub fn recognize_keyword_true(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start).copied() != Some('t') {
        return None;
    }
    if chars.get(start + 1).copied() != Some('r') {
        return None;
    }
    if chars.get(start + 2).copied() != Some('u') {
        return None;
    }
    if chars.get(start + 3).copied() != Some('e') {
        return None;
    }
    if !keyword_boundary(chars, start + 4) {
        return None;
    }
    Some(4)
}

/// q0 --f--> q1 --a--> q2 --l--> q3 --s--> q4 --e--> q5 (accept)
pub fn recognize_keyword_false(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start).copied() != Some('f') {
        return None;
    }
    if chars.get(start + 1).copied() != Some('a') {
        return None;
    }
    if chars.get(start + 2).copied() != Some('l') {
        return None;
    }
    if chars.get(start + 3).copied() != Some('s') {
        return None;
    }
    if chars.get(start + 4).copied() != Some('e') {
        return None;
    }
    if !keyword_boundary(chars, start + 5) {
        return None;
    }
    Some(5)
}

// --- Non-keyword recognizers ---

/// Recognize a standard identifier: [a-zA-Z][a-zA-Z0-9_]*
pub fn recognize_identifier(chars: &[char], start: usize) -> Option<usize> {
    // The `?` operator here means: if `get()` returns `None`, exit early with `None`.
    // In TS: `const first = arr[start]; if (first === undefined) return null;`
    let first_character = chars.get(start).copied()?;
    if !is_identifier_start(first_character) {
        return None;
    }

    // Greedy match: keep consuming while characters are identifier-continuation.
    let mut length = 1;
    while let Some(next_character) = chars.get(start + length).copied() {
        if is_identifier_continue(next_character) {
            length += 1;
        } else {
            break;
        }
    }

    Some(length)
}

/// Recognize an integer: [0-9]+
pub fn recognize_integer(chars: &[char], start: usize) -> Option<usize> {
    let first_character = chars.get(start).copied()?;
    if !is_integer_character(first_character) {
        return None;
    }

    // Greedy match: consume all consecutive digits.
    let mut length = 1;
    while let Some(next_character) = chars.get(start + length).copied() {
        if is_integer_character(next_character) {
            length += 1;
        } else {
            break;
        }
    }

    Some(length)
}

/// Recognize a wildcard token used by the example pattern matching code.
pub fn recognize_wildcard(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start).copied() == Some('_') {
        Some(1)
    } else {
        None
    }
}

/// DFA for string literals: "([a-zA-Z0-9_ ])*"
pub fn recognize_string(chars: &[char], start: usize) -> StringRecognition {
    // Must start with a double-quote character.
    if chars.get(start).copied() != Some('"') {
        return StringRecognition::Unterminated;
    }

    let mut index = start + 1;

    while let Some(next_character) = chars.get(index).copied() {
        if next_character == '"' {
            // Found the closing quote—success.
            return StringRecognition::Matched(index - start + 1);
        }

        if next_character == '\n' {
            // Hit end of line without closing — unterminated string.
            return StringRecognition::Unterminated;
        }

        if !is_string_character(next_character) {
            // Character not in the allowed set for strings.
            return StringRecognition::InvalidCharacter {
                character: next_character,
                offset: index - start,
            };
        }

        index += 1;
    }

    // Reached end of file without finding closing quote.
    StringRecognition::Unterminated
}

/// Recognize a comment from `#` until the end of the line.
pub fn recognize_comment(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start).copied() != Some('#') {
        return None;
    }

    // Consume everything until newline (or end of file).
    let mut index = start + 1;
    while let Some(next_character) = chars.get(index).copied() {
        if next_character == '\n' {
            break;
        }
        index += 1;
    }

    Some(index - start)
}

/// Recognize operators and punctuation that form token boundaries.
///
/// Multi-character operators (`==`, `!=`, `<=`, `>=`, `&&`, `||`, `=>`)
/// are checked BEFORE single-character ones, so we don't mistake `==` for
/// two separate `=` signs.
pub fn recognize_operator(chars: &[char], start: usize) -> Option<(TokenKind, usize)> {
    // `match` on the character at the current position.
    // `chars.get(start).copied()?` — if `None` at start, exit early.
    match chars.get(start).copied()? {
        // Single-character operators:
        '+' => Some((TokenKind::Plus, 1)),
        '-' => Some((TokenKind::Minus, 1)),
        '*' => Some((TokenKind::Star, 1)),
        '/' => Some((TokenKind::Slash, 1)),
        '%' => Some((TokenKind::Percent, 1)),

        // `=` → check for `==` (equal) or `=>` (fat arrow) first
        '=' => match chars.get(start + 1).copied() {
            Some('=') => Some((TokenKind::EqualEqual, 2)),
            Some('>') => Some((TokenKind::FatArrow, 2)),
            _ => Some((TokenKind::Assign, 1)),  // single `=`
        },

        // `!` → check for `!=` (not equal)
        '!' => match chars.get(start + 1).copied() {
            Some('=') => Some((TokenKind::NotEqual, 2)),
            _ => Some((TokenKind::Bang, 1)),  // single `!`
        },

        // `<` → check for `<=` (less than or equal)
        '<' => match chars.get(start + 1).copied() {
            Some('=') => Some((TokenKind::LessEqual, 2)),
            _ => Some((TokenKind::Less, 1)),  // single `<`
        },

        // `>` → check for `>=` (greater than or equal)
        '>' => match chars.get(start + 1).copied() {
            Some('=') => Some((TokenKind::GreaterEqual, 2)),
            _ => Some((TokenKind::Greater, 1)),  // single `>`
        },

        // `&` → only valid as `&&` (logical AND). Single `&` is NOT recognized.
        '&' => match chars.get(start + 1).copied() {
            Some('&') => Some((TokenKind::AndAnd, 2)),
            _ => None,
        },

        // `|` → only valid as `||` (logical OR). Single `|` is NOT recognized.
        '|' => match chars.get(start + 1).copied() {
            Some('|') => Some((TokenKind::OrOr, 2)),
            _ => None,
        },

        _ => None,  // Not an operator character
    }
}

/// Recognize delimiters used by the grammar.
pub fn recognize_delimiter(chars: &[char], start: usize) -> Option<(TokenKind, usize)> {
    match chars.get(start).copied()? {
        '(' => Some((TokenKind::LeftParen, 1)),
        ')' => Some((TokenKind::RightParen, 1)),
        '{' => Some((TokenKind::LeftBrace, 1)),
        '}' => Some((TokenKind::RightBrace, 1)),
        ',' => Some((TokenKind::Comma, 1)),
        _ => None,
    }
}
