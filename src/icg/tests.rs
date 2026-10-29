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
    fn var_decl_emits_assignment_quad() {
        // nyan x = 42
        //
        // The DECLARATION emits nothing, but the ASSIGNMENT does —
        // exactly like C where `int a;` generates no code and `a = 5;`
        // does. See the justification in src/icg/generator.rs.
        let program = Program {
            statements: vec![var_decl("x", int_lit("42"))],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        assert_eq!(result.quad_count, 1);
        assert!(result.quadruples.contains("="));
        assert!(result.quadruples.contains("42"));
        assert!(result.quadruples.contains("x"));
        // The keyword `nyan` must NEVER appear in the quadruple table.
        assert!(!result.quadruples.contains("nyan"));
    }

    #[test]
    fn simple_addition_emits_op_and_assignment() {
        // nyan x = a + b
        // → ( +, a, b, T1 )  then  ( =, T1, _, x )
        let program = Program {
            statements: vec![var_decl("x", add(ident("a"), ident("b")))],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        assert_eq!(result.quad_count, 2);
        let lines: Vec<&str> = result.quadruples.lines().collect();
        // First: the operation
        assert!(lines[0].contains("+"));
        assert!(lines[0].contains("a"));
        assert!(lines[0].contains("b"));
        assert!(lines[0].contains("T1"));
        // Second: the assignment of the result to the variable
        assert!(lines[1].contains("="));
        assert!(lines[1].contains("T1"));
        assert!(lines[1].contains("x"));
    }

    #[test]
    fn multiplication_chain_emits_correct_order() {
        // nyan x = a + b * c
        // → (*, b, c, T1) then (+, a, T1, T2) then (=, T2, _, x)
        let program = Program {
            statements: vec![var_decl(
                "x",
                add(ident("a"), mul(ident("b"), ident("c"))),
            )],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        let lines: Vec<&str> = result.quadruples.lines().collect();
        assert_eq!(lines.len(), 3);
        // First: multiplication
        assert!(lines[0].contains("*"));
        assert!(lines[0].contains("T1"));
        // Second: addition
        assert!(lines[1].contains("+"));
        assert!(lines[1].contains("T1"));
        assert!(lines[1].contains("T2"));
        // Third: assignment
        assert!(lines[2].contains("="));
        assert!(lines[2].contains("T2"));
        assert!(lines[2].contains("x"));
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
        // → ( >, a, b, T1 )  then  ( =, T1, _, r )
        let program = Program {
            statements: vec![var_decl("r", cmp_gt(ident("a"), ident("b")))],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        assert_eq!(result.quad_count, 2);
        assert!(result.quadruples.contains(">"));
    }

    #[test]
    fn if_else_emits_gotof_goto_labels() {
        // nyan r = if a > b { 1 } else { 2 }
        //
        // Expected sequence:
        //   ( >, a, b, T1 )          <- the condition
        //   ( GOTOF, T1, _, L1 )     <- false -> else
        //   ( =, 1, _, T2 )          <- then value
        //   ( GOTO, _, _, L3 )
        //   ( LABEL, _, _, L1 )      <- else label
        //   ( =, 2, _, T2 )          <- else value
        //   ( LABEL, _, _, L3 )      <- end label
        //   ( =, T2, _, r )          <- assignment
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
        assert!(result.quadruples.contains("GOTOF"));
        assert!(result.quadruples.contains("GOTO"));
        // Two LABEL lines (else + end)
        let label_count = result.quadruples.matches("LABEL").count();
        assert_eq!(label_count, 2);
        // The if-expression must yield a value: an assignment to the
        // result temporary must be present in BOTH branches.
        let assign_count = result
            .quadruples
            .lines()
            .filter(|l| l.contains("=") && !l.contains("=="))
            .count();
        // 1 (the then branch) + 1 (the else branch) + 1 (the final
        // assignment to `r`) = 3
        assert_eq!(assign_count, 3, "got: {}", result.quadruples);
    }

    #[test]
    fn if_without_else_initialises_result_to_zero() {
        // nyan r = if a > b { 1 }
        // The false path has no value, so the result temp is pre-set to 0.
        let program = Program {
            statements: vec![var_decl(
                "r",
                Box::new(Expr::If {
                    condition: cmp_gt(ident("a"), ident("b")),
                    then_branch: int_lit("1"),
                    else_branch: None,
                }),
            )],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        // There must be an assignment whose SOURCE is the literal 0
        // (the pre-initialisation of the result temporary).
        let has_zero_init = result.quadruples.lines().any(|l| {
            // Format: "  1  (      =,      0,      _,     T2 )"
            let fields: Vec<&str> = l.split(',').collect();
            fields.len() >= 2 && fields[0].contains('=') && fields[1].trim() == "0"
        });
        assert!(
            has_zero_init,
            "expected a zero-initialisation, got: {}",
            result.quadruples
        );
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
        // 2 condition tests (==, GOTOV) for the non-default arms
        assert_eq!(result.quadruples.matches("==").count(), 2);
        assert_eq!(result.quadruples.matches("GOTOV").count(), 2);
        // 3 arm labels + 1 end label
        assert_eq!(result.quadruples.matches("LABEL").count(), 4);
        // Each of the 3 arms assigns its value to the result temp, plus
        // the final assignment to `r` = 4 assignments.
        let assign_count = result
            .quadruples
            .lines()
            .filter(|l| l.contains('=') && !l.contains("=="))
            .count();
        assert_eq!(assign_count, 4, "got: {}", result.quadruples);
    }

    #[test]
    fn match_result_temp_is_shared_by_all_arms() {
        // All three arms must write to the SAME result temporary, so the
        // value of the match is well defined regardless of which arm ran.
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
                            pattern: Pattern::Wildcard,
                            body: str_lit("F"),
                        },
                    ],
                }),
            )],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        // The two arm assignments and the final one must all target the
        // same temporary.
        let targets: Vec<&str> = result
            .quadruples
            .lines()
            .filter(|l| l.contains('=') && !l.contains("=="))
            .filter_map(|l| l.rsplit(',').next())
            .map(|s| s.trim().trim_end_matches(')').trim())
            .collect();
        assert!(targets.len() >= 3, "got: {:?}", targets);
        // The first two are the arm assignments (same temp), the last one
        // is the assignment to the variable `r`.
        assert_eq!(targets[0], targets[1], "arms must share a temp");
        assert_eq!(targets[2], "r");
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
    fn bool_literals_are_recognised() {
        // nyan b = true   →  the literal is used directly, no temp needed.
        let program = Program {
            statements: vec![var_decl("b", true_lit())],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        // The assignment quadruple carries the literal as the source.
        assert!(result.quadruples.contains("true"));
        assert!(result.quadruples.contains("b"));

        // The same for `false`.
        let program = Program {
            statements: vec![var_decl("b", false_lit())],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        assert!(result.quadruples.contains("false"));
    }

    #[test]
    fn both_quadruples_and_triples_emitted() {
        // Same program should produce equal numbers of quads and triples
        // (1-to-1 correspondence) — every emit_quad is paired with an
        // emit_triple.
        let program = Program {
            statements: vec![var_decl("x", add(ident("a"), ident("b")))],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        assert_eq!(result.quad_count, 2);
        assert_eq!(result.triple_count, 2);
    }

    #[test]
    fn quadruples_and_triples_stay_in_sync_for_match() {
        // A match is the most complex construct; verify the 1-to-1
        // correspondence holds there too.
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
                            pattern: Pattern::Wildcard,
                            body: str_lit("F"),
                        },
                    ],
                }),
            )],
        };
        let gen = ICGenerator::new();
        let result = gen.generate(&program);
        assert_eq!(result.quad_count, result.triple_count);
    }
}
