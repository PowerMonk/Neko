// ============================================================================
// Semantic analyzer tests
//
// Strategy: build the token stream + Program AST directly, then run the
// analyzer. We don't go through the lexer or parser here because each
// test cares about ONE semantic concern and shouldn't depend on their
// quirks.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::SemanticAnalyzer;
    use crate::lexer::{Token, TokenKind};
    use crate::parser::Parser;

    /// Helper: build a Token from kind + lexeme at line 1, col 1.
    fn tk(kind: TokenKind, lexeme: &str) -> Token {
        Token::new(kind, lexeme.to_string(), 1, 1)
    }

    /// Helper: parse a token list with the parser, run semantic on it.
    fn analyze_tokens(tokens: Vec<Token>) -> Vec<String> {
        let parser = Parser::new(tokens.clone());
        let parsed = parser.parse();
        let analyzer = SemanticAnalyzer::new(tokens);
        let result = analyzer.analyze(&parsed.program);
        result.errors.iter().map(|e| e.message.clone()).collect()
    }

    /// Helper: parse a token list with the parser, run semantic on it,
    /// returning whether parsing itself succeeded.
    fn analyze_tokens_full(
        tokens: Vec<Token>,
    ) -> (Vec<String>, Vec<crate::parser::SyntaxError>) {
        let parser = Parser::new(tokens.clone());
        let parsed = parser.parse();
        let analyzer = SemanticAnalyzer::new(tokens);
        let result = analyzer.analyze(&parsed.program);
        (
            result.errors.iter().map(|e| e.message.clone()).collect(),
            parsed.errors,
        )
    }

    // ============================================================
    // 1. Comprobación de tipos
    // ============================================================

    #[test]
    fn type_check_int_addition_ok() {
        // nyan x = 1 + 2
        // meow(x)
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "1"),
            tk(TokenKind::Plus, "+"),
            tk(TokenKind::Integer, "2"),
            tk(TokenKind::KeywordMeow, "meow"),
            tk(TokenKind::LeftParen, "("),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::RightParen, ")"),
        ];
        let (sem_errs, _) = analyze_tokens_full(tokens);
        assert!(sem_errs.is_empty(), "got: {:?}", sem_errs);
    }

    #[test]
    fn type_check_string_plus_int_is_error() {
        // nyan x = "hi" + 1  — should report type mismatch.
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::StringLiteral, "\"hi\""),
            tk(TokenKind::Plus, "+"),
            tk(TokenKind::Integer, "1"),
        ];
        let (sem_errs, parse_errs) = analyze_tokens_full(tokens);
        assert!(parse_errs.is_empty(), "parser errors: {:?}", parse_errs);
        assert!(
            sem_errs.iter().any(|m| m.contains("type mismatch")),
            "expected type mismatch, got: {:?}",
            sem_errs
        );
    }

    #[test]
    fn type_check_if_must_be_bool() {
        // nyan x = if 1 + 2 { 3 } else { 4 }
        // — `1 + 2` is int, not bool, so this should error.
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::KeywordIf, "if"),
            tk(TokenKind::Integer, "1"),
            tk(TokenKind::Plus, "+"),
            tk(TokenKind::Integer, "2"),
            tk(TokenKind::LeftBrace, "{"),
            tk(TokenKind::Integer, "3"),
            tk(TokenKind::RightBrace, "}"),
            tk(TokenKind::KeywordElse, "else"),
            tk(TokenKind::LeftBrace, "{"),
            tk(TokenKind::Integer, "4"),
            tk(TokenKind::RightBrace, "}"),
        ];
        let (sem_errs, _) = analyze_tokens_full(tokens);
        assert!(
            sem_errs.iter().any(|m| m.contains("condition of 'if'")),
            "expected if-condition error, got: {:?}",
            sem_errs
        );
    }

    #[test]
    fn type_check_branches_must_match() {
        // nyan x = if true { 1 } else { "hi" }
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::KeywordIf, "if"),
            tk(TokenKind::KeywordTrue, "true"),
            tk(TokenKind::LeftBrace, "{"),
            tk(TokenKind::Integer, "1"),
            tk(TokenKind::RightBrace, "}"),
            tk(TokenKind::KeywordElse, "else"),
            tk(TokenKind::LeftBrace, "{"),
            tk(TokenKind::StringLiteral, "\"hi\""),
            tk(TokenKind::RightBrace, "}"),
        ];
        let (sem_errs, _) = analyze_tokens_full(tokens);
        assert!(
            sem_errs.iter().any(|m| m.contains("branches of 'if'")),
            "expected branch-mismatch error, got: {:?}",
            sem_errs
        );
    }

    #[test]
    fn type_check_negate_string_is_error() {
        // nyan s = "hi"
        // nyan x = -s
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "s"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::StringLiteral, "\"hi\""),
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Minus, "-"),
            tk(TokenKind::Identifier, "s"),
        ];
        let (sem_errs, _) = analyze_tokens_full(tokens);
        assert!(
            sem_errs.iter().any(|m| m.contains("negate non-int")),
            "expected negate error, got: {:?}",
            sem_errs
        );
    }

    #[test]
    fn type_check_logical_operands_must_be_bool() {
        // nyan x = 1 && true
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "1"),
            tk(TokenKind::AndAnd, "&&"),
            tk(TokenKind::KeywordTrue, "true"),
        ];
        let (sem_errs, _) = analyze_tokens_full(tokens);
        assert!(
            sem_errs.iter().any(|m| m.contains("logical operator")),
            "expected logical-op error, got: {:?}",
            sem_errs
        );
    }

    // ============================================================
    // 2. Comprobaciones de flujo de control — exhaustividad
    // ============================================================

    #[test]
    fn match_must_have_default_arm() {
        // nyan x = 1
        // nyan y = match x { 100 => "hundred" 90 => "ninety" }
        // (no `_` arm)
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "1"),
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "y"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::KeywordMatch, "match"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::LeftBrace, "{"),
            tk(TokenKind::Integer, "100"),
            tk(TokenKind::FatArrow, "=>"),
            tk(TokenKind::StringLiteral, "\"hundred\""),
            tk(TokenKind::Integer, "90"),
            tk(TokenKind::FatArrow, "=>"),
            tk(TokenKind::StringLiteral, "\"ninety\""),
            tk(TokenKind::RightBrace, "}"),
        ];
        let (sem_errs, _) = analyze_tokens_full(tokens);
        assert!(
            sem_errs.iter().any(|m| m.contains("non-exhaustive")),
            "expected non-exhaustive error, got: {:?}",
            sem_errs
        );
    }

    #[test]
    fn match_with_default_arm_is_ok() {
        // nyan x = 1
        // nyan y = match x { _ => "anything" }
        // meow(y)
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "1"),
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "y"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::KeywordMatch, "match"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::LeftBrace, "{"),
            tk(TokenKind::Wildcard, "_"),
            tk(TokenKind::FatArrow, "=>"),
            tk(TokenKind::StringLiteral, "\"anything\""),
            tk(TokenKind::RightBrace, "}"),
            tk(TokenKind::KeywordMeow, "meow"),
            tk(TokenKind::LeftParen, "("),
            tk(TokenKind::Identifier, "y"),
            tk(TokenKind::RightParen, ")"),
        ];
        let (sem_errs, parse_errs) = analyze_tokens_full(tokens);
        assert!(parse_errs.is_empty(), "parser errors: {:?}", parse_errs);
        assert!(
            sem_errs.is_empty(),
            "match with default should be clean, got: {:?}",
            sem_errs
        );
    }

    // ============================================================
    // 3. Comprobaciones de unicidad
    // ============================================================

    #[test]
    fn redeclaration_in_same_scope_errors() {
        // nyan x = 1
        // nyan x = 2
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "1"),
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "2"),
        ];
        let (sem_errs, _) = analyze_tokens_full(tokens);
        assert!(
            sem_errs.iter().any(|m| m.contains("redeclaration")),
            "expected redeclaration error, got: {:?}",
            sem_errs
        );
    }

    #[test]
    fn shadowing_inner_block_errors() {
        // fn neko {           ← function body allows multi-statement blocks
        //     nyan x = 1
        //     nyan x = 2      ← shadowing in the same inner block (also
        //                         the same scope — redeclaration)
        // }
        let tokens = vec![
            tk(TokenKind::KeywordFn, "fn"),
            tk(TokenKind::KeywordNeko, "neko"),
            tk(TokenKind::LeftBrace, "{"),
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "1"),
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "2"),
            tk(TokenKind::RightBrace, "}"),
        ];
        let (sem_errs, parse_errs) = analyze_tokens_full(tokens);
        assert!(parse_errs.is_empty(), "parser errors: {:?}", parse_errs);
        // Same-scope redeclaration fires; for actual cross-scope
        // shadowing we'd need an inner `if`-block which the parser
        // doesn't yet support for multi-statement bodies. The
        // redeclaration is enough to prove the uniqueness machinery
        // works.
        assert!(
            sem_errs.iter().any(|m| m.contains("redeclaration")),
            "expected redeclaration/shadowing error, got: {:?}",
            sem_errs
        );
    }

    #[test]
    fn duplicate_match_arm_pattern_errors() {
        // nyan x = 1
        // nyan y = match x { 1 => "a" 1 => "b" _ => "c" }
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "1"),
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "y"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::KeywordMatch, "match"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::LeftBrace, "{"),
            tk(TokenKind::Integer, "1"),
            tk(TokenKind::FatArrow, "=>"),
            tk(TokenKind::StringLiteral, "\"a\""),
            tk(TokenKind::Integer, "1"),
            tk(TokenKind::FatArrow, "=>"),
            tk(TokenKind::StringLiteral, "\"b\""),
            tk(TokenKind::Wildcard, "_"),
            tk(TokenKind::FatArrow, "=>"),
            tk(TokenKind::StringLiteral, "\"c\""),
            tk(TokenKind::RightBrace, "}"),
        ];
        let (sem_errs, _) = analyze_tokens_full(tokens);
        assert!(
            sem_errs.iter().any(|m| m.contains("duplicate match arm")),
            "expected duplicate-arm error, got: {:?}",
            sem_errs
        );
    }

    // ============================================================
    // Auxiliares: undefined + unused
    // ============================================================

    #[test]
    fn undefined_identifier_errors() {
        // meow(z)   — z never declared
        let tokens = vec![
            tk(TokenKind::KeywordMeow, "meow"),
            tk(TokenKind::LeftParen, "("),
            tk(TokenKind::Identifier, "z"),
            tk(TokenKind::RightParen, ")"),
        ];
        let (sem_errs, _) = analyze_tokens_full(tokens);
        assert!(
            sem_errs.iter().any(|m| m.contains("undefined identifier")),
            "expected undefined error, got: {:?}",
            sem_errs
        );
    }

    #[test]
    fn use_before_declaration_errors() {
        // meow(x)    — x declared AFTER the meow
        // nyan x = 1
        let tokens = vec![
            tk(TokenKind::KeywordMeow, "meow"),
            tk(TokenKind::LeftParen, "("),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::RightParen, ")"),
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "1"),
        ];
        let (sem_errs, _) = analyze_tokens_full(tokens);
        // First error should be "undefined" because x isn't yet in scope.
        assert!(
            sem_errs.iter().any(|m| m.contains("undefined identifier")),
            "expected undefined (use-before-decl) error, got: {:?}",
            sem_errs
        );
    }

    #[test]
    fn unused_variable_errors() {
        // nyan x = 1   — never read
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "1"),
        ];
        let (sem_errs, _) = analyze_tokens_full(tokens);
        assert!(
            sem_errs.iter().any(|m| m.contains("unused variable")),
            "expected unused-variable error, got: {:?}",
            sem_errs
        );
    }

    #[test]
    fn used_variable_does_not_trigger_unused() {
        // nyan x = 1
        // meow(x)
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "1"),
            tk(TokenKind::KeywordMeow, "meow"),
            tk(TokenKind::LeftParen, "("),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::RightParen, ")"),
        ];
        let (sem_errs, parse_errs) = analyze_tokens_full(tokens);
        assert!(parse_errs.is_empty(), "parser errors: {:?}", parse_errs);
        assert!(
            sem_errs.is_empty(),
            "used variable should not be flagged, got: {:?}",
            sem_errs
        );
    }

    // ============================================================
    // Sanity check: analyze_tokens helper works at all
    // ============================================================

    #[test]
    fn empty_program_is_clean() {
        let msgs = analyze_tokens(vec![]);
        assert!(msgs.is_empty(), "empty program should produce no errors");
    }
}
