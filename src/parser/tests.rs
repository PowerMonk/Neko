// ============================================================================
// Parser tests
//
// Strategy: feed pre-built token lists directly into the parser, bypassing
// the file system and the lexer. This keeps the tests focused on parser
// behavior and lets them run in milliseconds.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::ast::{Expr, Literal, Stmt};
    use super::super::parser::Parser;
    use crate::lexer::{Token, TokenKind};

    /// Helper: build a Token from kind + lexeme at line 1, col 1.
    fn tk(kind: TokenKind, lexeme: &str) -> Token {
        Token::new(kind, lexeme.to_string(), 1, 1)
    }

    #[test]
    fn parses_empty_program() {
        let result = Parser::new(vec![]).parse();
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.program.statements.len(), 0);
    }

    #[test]
    fn parses_single_var_decl() {
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "42"),
        ];
        let result = Parser::new(tokens).parse();
        assert_eq!(result.errors.len(), 0, "errors: {:?}", result.errors);
        assert_eq!(result.program.statements.len(), 1);
        match &result.program.statements[0] {
            Stmt::VarDecl { name, .. } => assert_eq!(name, "x"),
            other => panic!("expected VarDecl, got {:?}", other),
        }
    }

    #[test]
    fn parses_if_with_else() {
        // nyan x = if true { 1 } else { 2 }
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
            tk(TokenKind::Integer, "2"),
            tk(TokenKind::RightBrace, "}"),
        ];
        let result = Parser::new(tokens).parse();
        assert_eq!(result.errors.len(), 0, "errors: {:?}", result.errors);
        assert_eq!(result.program.statements.len(), 1);
        match &result.program.statements[0] {
            Stmt::VarDecl { value, .. } => match value.as_ref() {
                Expr::If { else_branch: Some(_), .. } => {}
                other => panic!("expected If with else inside VarDecl, got {:?}", other),
            },
            other => panic!("expected VarDecl, got {:?}", other),
        }
    }

    #[test]
    fn parses_match_with_three_arms() {
        // nyan result = match score { 100 => "Perfect" 90 => "Excellent" _ => "Fail" }
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "result"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::KeywordMatch, "match"),
            tk(TokenKind::Identifier, "score"),
            tk(TokenKind::LeftBrace, "{"),
            tk(TokenKind::Integer, "100"),
            tk(TokenKind::FatArrow, "=>"),
            tk(TokenKind::StringLiteral, "\"Perfect\""),
            tk(TokenKind::Integer, "90"),
            tk(TokenKind::FatArrow, "=>"),
            tk(TokenKind::StringLiteral, "\"Excellent\""),
            tk(TokenKind::Wildcard, "_"),
            tk(TokenKind::FatArrow, "=>"),
            tk(TokenKind::StringLiteral, "\"Fail\""),
            tk(TokenKind::RightBrace, "}"),
        ];
        let result = Parser::new(tokens).parse();
        assert_eq!(result.errors.len(), 0, "errors: {:?}", result.errors);
    }

    #[test]
    fn reports_missing_paren_in_meow() {
        let tokens = vec![
            tk(TokenKind::KeywordMeow, "meow"),
            tk(TokenKind::LeftParen, "("),
            tk(TokenKind::Identifier, "x"),
            // no closing paren, no further tokens
        ];
        let result = Parser::new(tokens).parse();
        assert!(
            result.errors.iter().any(|e| e.message.contains("')'")),
            "expected an error about missing ')', got: {:?}",
            result.errors
        );
    }

    #[test]
    fn recovers_and_continues_after_bad_token() {
        // Garbage token before a valid VarDecl. Parser should report the
        // garbage and still parse the VarDecl.
        let tokens = vec![
            tk(TokenKind::Integer, "999"), // unexpected
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Integer, "1"),
        ];
        let result = Parser::new(tokens).parse();
        assert!(!result.errors.is_empty(), "should report the garbage");
        // The VarDecl should still have been parsed.
        assert!(
            result.program.statements.iter().any(|s| matches!(s, Stmt::VarDecl { .. })),
            "expected VarDecl to be parsed despite leading garbage"
        );
    }

    #[test]
    fn parses_addition_chain() {
        // nyan x = a + b - c  →  Add chain with 2 ops (+ and -).
        let tokens = vec![
            tk(TokenKind::KeywordNyan, "nyan"),
            tk(TokenKind::Identifier, "x"),
            tk(TokenKind::Assign, "="),
            tk(TokenKind::Identifier, "a"),
            tk(TokenKind::Plus, "+"),
            tk(TokenKind::Identifier, "b"),
            tk(TokenKind::Minus, "-"),
            tk(TokenKind::Identifier, "c"),
        ];
        let result = Parser::new(tokens).parse();
        assert_eq!(result.errors.len(), 0, "errors: {:?}", result.errors);
        if let Stmt::VarDecl { value, .. } = &result.program.statements[0] {
            // Drill past Or/And layers to find the Add chain.
            let inner = unwrap_chain_layers(value.as_ref());
            match inner {
                Expr::Add(chain) => {
                    assert_eq!(chain.ops.len(), 2);
                    assert_eq!(chain.operands.len(), 2);
                }
                other => panic!("expected Add chain, got {:?}", other),
            }
        } else {
            panic!("expected VarDecl");
        }
    }

    #[test]
    fn parses_unary_negation() {
        // meow( -5 )
        // Path through AST (each level has empty ops/operands):
        //   PrintStmt → Or → And → Add → Mul → Unary(Negate) → Primary(Int(5))
        let tokens = vec![
            tk(TokenKind::KeywordMeow, "meow"),
            tk(TokenKind::LeftParen, "("),
            tk(TokenKind::Minus, "-"),
            tk(TokenKind::Integer, "5"),
            tk(TokenKind::RightParen, ")"),
        ];
        let result = Parser::new(tokens).parse();
        assert_eq!(result.errors.len(), 0, "errors: {:?}", result.errors);
        if let Stmt::PrintStmt { value } = &result.program.statements[0] {
            let inner = unwrap_chain_layers(value.as_ref());
            match inner {
                Expr::Negate { operand } => match operand.as_ref() {
                    Expr::Primary(Literal::Int(s)) => assert_eq!(s, "5"),
                    _ => panic!("expected Int inside Negate"),
                },
                other => panic!("expected Negate after unwrapping, got {:?}", other),
            }
        } else {
            panic!("expected PrintStmt");
        }
    }

    /// Drill through Or → And → Add → Mul → Cmp layers to find the
    /// actual interesting expression at the bottom. Each layer is a
    /// single-operand chain in tests like these.
    fn unwrap_chain_layers(e: &Expr) -> &Expr {
        let mut cur = e;
        loop {
            cur = match cur {
                Expr::Or(c) if c.ops.is_empty() => &c.left,
                Expr::And(c) if c.ops.is_empty() => &c.left,
                Expr::Compare { left, op: None, .. } => left,
                Expr::Add(c) if c.ops.is_empty() => &c.left,
                Expr::Mul(c) if c.ops.is_empty() => &c.left,
                _ => return cur,
            };
        }
    }
}
