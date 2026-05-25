/// Character-level checks for the Neko alphabet.
///
/// The lexer keeps these checks separate from lexeme recognition so the project
/// can clearly show the difference between allowed source characters and token
/// patterns.

pub fn is_valid_source_character(character: char) -> bool {
    character.is_ascii_graphic() || character.is_ascii_whitespace()
}

pub fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic()
}

pub fn is_identifier_continue(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

pub fn is_integer_character(character: char) -> bool {
    character.is_ascii_digit()
}

pub fn is_string_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_' || character == ' '
}

pub fn is_wildcard(character: char) -> bool {
    character == '_'
}

pub fn is_operator_start(character: char) -> bool {
    matches!(
        character,
        '+' | '-' | '*' | '/' | '%' | '=' | '!' | '<' | '>' | '&' | '|'
    )
}

pub fn is_delimiter(character: char) -> bool {
    matches!(character, '(' | ')' | '{' | '}' | ',' | ';')
}
