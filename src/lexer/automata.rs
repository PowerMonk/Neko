use super::alphabet::{
    is_identifier_continue, is_identifier_start, is_integer_character, is_string_character,
};
use super::token::TokenKind;

// Result of running the string DFA.
pub enum StringRecognition {
    Matched(usize),
    Unterminated,
    InvalidCharacter { character: char, offset: usize },
}

fn keyword_boundary(chars: &[char], end_index: usize) -> bool {
    match chars.get(end_index).copied() {
        Some(next_character) => !is_identifier_continue(next_character),
        None => true,
    }
}

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

/// Recognize a standard identifier: [a-zA-Z][a-zA-Z0-9_]*
pub fn recognize_identifier(chars: &[char], start: usize) -> Option<usize> {
    let first_character = chars.get(start).copied()?;
    if !is_identifier_start(first_character) {
        return None;
    }

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

/// DFA for strings: "([a-zA-Z0-9_ ])*"
pub fn recognize_string(chars: &[char], start: usize) -> StringRecognition {
    if chars.get(start).copied() != Some('"') {
        return StringRecognition::Unterminated;
    }

    let mut index = start + 1;

    while let Some(next_character) = chars.get(index).copied() {
        if next_character == '"' {
            return StringRecognition::Matched(index - start + 1);
        }

        if next_character == '\n' {
            return StringRecognition::Unterminated;
        }

        if !is_string_character(next_character) {
            return StringRecognition::InvalidCharacter {
                character: next_character,
                offset: index - start,
            };
        }

        index += 1;
    }

    StringRecognition::Unterminated
}

/// Recognize a comment from `#` until the end of the line.
pub fn recognize_comment(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start).copied() != Some('#') {
        return None;
    }

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
pub fn recognize_operator(chars: &[char], start: usize) -> Option<(TokenKind, usize)> {
    match chars.get(start).copied()? {
        '+' => Some((TokenKind::Plus, 1)),
        '-' => Some((TokenKind::Minus, 1)),
        '*' => Some((TokenKind::Star, 1)),
        '/' => Some((TokenKind::Slash, 1)),
        '%' => Some((TokenKind::Percent, 1)),
        '=' => match chars.get(start + 1).copied() {
            Some('=') => Some((TokenKind::EqualEqual, 2)),
            Some('>') => Some((TokenKind::FatArrow, 2)),
            _ => Some((TokenKind::Assign, 1)),
        },
        '!' => match chars.get(start + 1).copied() {
            Some('=') => Some((TokenKind::NotEqual, 2)),
            _ => Some((TokenKind::Bang, 1)),
        },
        '<' => match chars.get(start + 1).copied() {
            Some('=') => Some((TokenKind::LessEqual, 2)),
            _ => Some((TokenKind::Less, 1)),
        },
        '>' => match chars.get(start + 1).copied() {
            Some('=') => Some((TokenKind::GreaterEqual, 2)),
            _ => Some((TokenKind::Greater, 1)),
        },
        '&' => match chars.get(start + 1).copied() {
            Some('&') => Some((TokenKind::AndAnd, 2)),
            _ => None,
        },
        '|' => match chars.get(start + 1).copied() {
            Some('|') => Some((TokenKind::OrOr, 2)),
            _ => None,
        },
        _ => None,
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
