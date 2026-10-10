// ============================================================================
// ICG unit tests
//
// Strategy: build a Program AST by hand (no lexer, no parser), feed it to
// the ICGenerator, and assert on the resulting quadruples and triples by
// string match. Hand-built ASTs let us test the generator in isolation.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::emitter::CodeEmitter;
    use super::super::quadruple::Quadruple;
    use super::super::triple::Triple;
    use super::super::ICGenerator;
    use crate::parser::ast::*;

    // ---- helpers to build AST nodes concisely ----

    fn int_lit(s: &str) -> Box<Expr> {
        Box::new(Expr::Primary(Literal::Int(s.to_string())))
    }

    fn str_lit(s: &str) -> Box<Expr> {
        Box::new(Expr::Primary(Literal::Str(s.to_string())))
    }

    fn ident(name: &str) -> Box<Expr> {
        Box::new(Expr::Primary(Literal::Ident(name.to_string())))
    }

    fn true_lit() -> Box<Expr> {
        Box::new(Expr::Primary(Literal::True))
    }

    fn false_lit() -> Box<Expr> {
        Box::new(Expr::Primary(Literal::False))
    }

    fn add(left: Box<Expr>, right: Box<Expr>) -> Box<Expr> {
        Box::new(Expr::Add(Chain {
            left,
            ops: vec![AddOpChain::Plus],
            operands: vec![*right],
        }))
    }

    fn mul(left: Box<Expr>, right: Box<Expr>) -> Box<Expr> {
        Box::new(Expr::Mul(Chain {
            left,
            ops: vec![MulOpChain::Times],
            operands: vec![*right],
        }))
    }

    fn cmp_gt(left: Box<Expr>, right: Box<Expr>) -> Box<Expr> {
        Box::new(Expr::Compare {
            left,
            op: Some(CmpOp::Gt),
            right: Some(right),
        })
    }

    fn var_decl(name: &str, value: Box<Expr>) -> Stmt {
        Stmt::VarDecl {
            name: name.to_string(),
            value,
        }
    }

    fn print_stmt(value: Box<Expr>) -> Stmt {
        Stmt::PrintStmt { value }
    }

    fn fn_decl(body: Vec<Stmt>) -> Stmt {
        Stmt::FnDecl { body }
    }

    // ---- CodeEmitter (unit) ----

    #[test]
    fn emitter_temps_start_at_t1() {
        let mut e = CodeEmitter::new();
        assert_eq!(e.next_temp(), "T1");
        assert_eq!(e.next_temp(), "T2");
        assert_eq!(e.next_temp(), "T3");
    }

    #[test]
    fn emitter_labels_start_at_l1() {
        let mut e = CodeEmitter::new();
        assert_eq!(e.next_label(), "L1");
        assert_eq!(e.next_label(), "L2");
    }

    #[test]
    fn emitter_triple_index_is_zero_based() {
        let mut e = CodeEmitter::new();
        assert_eq!(e.emit_triple("+", "a", "b"), 0);
        assert_eq!(e.emit_triple("*", "c", "d"), 1);
        assert_eq!(e.emit_triple("PRINT", "x", "_"), 2);
    }

    #[test]
    fn quadruple_render_columns_aligned() {
        let q = Quadruple::new("+", "a", "b", "T1");
        let s = q.render();
        // Should contain all four fields in the ( op , arg1 , arg2 , result ) layout.
        assert!(s.contains("+"));
        assert!(s.contains("a"));
        assert!(s.contains("b"));
        assert!(s.contains("T1"));
        assert!(s.starts_with(" ("));
        assert!(s.ends_with(" )"));
    }

    #[test]
    fn triple_render_includes_index() {
        let t = Triple::new("+", "a", "b");
        let s = t.render(0);
        assert!(s.contains("(  0)"));
        assert!(s.contains("+"));
    }

    // ---- ICGenerator on hand-built ASTs ----

    #[test]
    fn empty_program_emits_nothing() {
        let program = Program { statements: vec![] };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        assert_eq!(result.quad_count, 0);
        assert_eq!(result.triple_count, 0);
    }

    #[test]
    fn var_decl_does_not_emit_quads() {
        // nyan x = 42   — RHS is a literal, no quadruple emitted.
        let program = Program {
            statements: vec![var_decl("x", int_lit("42"))],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        assert_eq!(result.quad_count, 0);
    }

    #[test]
    fn simple_addition_emits_one_quad() {
        // nyan x = a + b
        let program = Program {
            statements: vec![var_decl("x", add(ident("a"), ident("b")))],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        assert_eq!(result.quad_count, 1);
        // ( + , a , b , T1 )
        assert!(result.quadruples.contains("+"));
        assert!(result.quadruples.contains("a"));
        assert!(result.quadruples.contains("b"));
        assert!(result.quadruples.contains("T1"));
    }

    #[test]
    fn multiplication_chain_emits_correct_order() {
        // nyan x = a + b * c
        // Should emit: (*, b, c, T1) then (+, a, T1, T2)
        let program = Program {
            statements: vec![var_decl(
                "x",
                add(ident("a"), mul(ident("b"), ident("c"))),
            )],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        let lines: Vec<&str> = result.quadruples.lines().collect();
        assert_eq!(lines.len(), 2);
        // First: multiplication
        assert!(lines[0].contains("*"));
        assert!(lines[0].contains("T1"));
        // Second: addition
        assert!(lines[1].contains("+"));
        assert!(lines[1].contains("T1"));
        assert!(lines[1].contains("T2"));
    }

    #[test]
    fn print_stmt_emits_print_quad() {
        // meow(x)
        let program = Program {
            statements: vec![print_stmt(ident("x"))],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        assert_eq!(result.quad_count, 1);
        assert!(result.quadruples.contains("PRINT"));
        assert!(result.quadruples.contains("x"));
    }

    #[test]
    fn comparison_emits_op_quad() {
        // nyan r = a > b
        let program = Program {
            statements: vec![var_decl("r", cmp_gt(ident("a"), ident("b")))],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        assert_eq!(result.quad_count, 1);
        assert!(result.quadruples.contains(">"));
    }

    #[test]
    fn if_else_emits_gotof_goto_labels() {
        // nyan r = if a > b { 1 } else { 2 }
        let program = Program {
            statements: vec![var_decl(
                "r",
                Box::new(Expr::If {
                    condition: cmp_gt(ident("a"), ident("b")),
                    then_branch: int_lit("1"),
                    else_branch: Some(int_lit("2")),
                }),
            )],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        // 1 comparison + (GOTOF, GOTO, LABEL, LABEL) = 5 quadruples
        assert_eq!(result.quad_count, 5);
        assert!(result.quadruples.contains("GOTOF"));
        assert!(result.quadruples.contains("GOTO"));
        // Two LABEL lines
        let label_count = result.quadruples.matches("LABEL").count();
        assert_eq!(label_count, 2);
    }

    #[test]
    fn match_with_three_arms_emits_correct_structure() {
        // nyan r = match score { 100 => "P" 90 => "E" _ => "F" }
        let program = Program {
            statements: vec![var_decl(
                "r",
                Box::new(Expr::Match {
                    scrutinee: ident("score"),
                    arms: vec![
                        Arm {
                            pattern: Pattern::Int("100".to_string()),
                            body: str_lit("P"),
                        },
                        Arm {
                            pattern: Pattern::Int("90".to_string()),
                            body: str_lit("E"),
                        },
                        Arm {
                            pattern: Pattern::Wildcard,
                            body: str_lit("F"),
                        },
                    ],
                }),
            )],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        // 2 condition tests (==, GOTOV) for non-default arms, each followed by
        // body. Bodies for string literals don't emit anything.
        // Structure: ==, GOTOV, ==, GOTOV, LABEL, GOTO, LABEL, GOTO, LABEL, LABEL
        // = 10 quadruples
        assert!(result.quad_count >= 9);
        assert_eq!(result.quadruples.matches("==").count(), 2);
        assert_eq!(result.quadruples.matches("GOTOV").count(), 2);
        // 3 arm labels + 1 end label
        assert_eq!(result.quadruples.matches("LABEL").count(), 4);
    }

    #[test]
    fn fn_decl_emits_only_body_quads() {
        // fn neko { meow(x) }
        let program = Program {
            statements: vec![fn_decl(vec![print_stmt(ident("x"))])],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        // Only the meow, not the fn neko itself.
        assert_eq!(result.quad_count, 1);
        assert!(result.quadruples.contains("PRINT"));
    }

    #[test]
    fn both_quadruples_and_triples_emitted() {
        // Same program should produce equal numbers of quads and triples
        // (1-to-1 correspondence).
        let program = Program {
            statements: vec![var_decl(
                "x",
                add(ident("a"), ident("b")),
            )],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        assert_eq!(result.quad_count, 1);
        assert_eq!(result.triple_count, 1);
    }
}
