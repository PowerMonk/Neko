// ============================================================================
// ICGenerator — Intermediate Code Generator
//
// Translates a Neko `Program` AST into three-address-code (3AC) tuples
// in two formats: quadruples (4 fields) and triples (3 fields with
// positional references).
//
// Visitor pattern
// ---------------
// `generate_expr` returns a `Value` — an enum describing "where the
// result of this expression lives". Most expressions produce a
// `Value::Temp(String)` (e.g. `T1`), but:
//
//   - literals and identifiers produce `Value::Operand(String)` (the
//     literal text or the variable name); no quadruple is emitted,
//     the value is used directly by the parent expression.
//
//   - control flow (`if`, `match`) returns `Value::Operand("_")`
//     because there is no single value to return (the result depends
//     on which branch ran). The caller of an `if` inside another
//     expression is not supported in Neko.
//
// Design choices
// --------------
//   - Both quadruples and triples are produced in a SINGLE pass. The
//     emitter carries two independent lists and counters.
//   - The ICG does NOT do any semantic validation. The contract is:
//     "if you got here, the program is already correct".
//   - `fn neko { ... }` emits only the body; the function wrapper
//     does not generate any tuple.
//   - Block delimiters (`{`, `}`) do not generate tuples.
//
// The assignment quadruple (`=`)
// ------------------------------
// `nyan x = expr` produces TWO kinds of output, and the distinction
// matters:
//
//   1. The DECLARATION produces nothing. There is no `DECLARE`
//      quadruple; the keyword `nyan` never appears in the quadruple
//      table. This mirrors C, where `int a;` (a pure declaration)
//      generates no code.
//
//   2. The ASSIGNMENT produces one quadruple: `(=, <src>, _, x)`.
//      This mirrors C, where `a = 5;` (an assignment) DOES generate
//      code. See Aho/Sethi/Ullman, "Dragon Book" §6.7 ("Translation of
//      Statements"), where `x = y + z` becomes:
//
//          t1 = y + z
//          x  = t1
//
// Why the assignment must live here and not in the code generator:
// the quadruple table is a FLAT list. Only this generator knows that
// the temporary holding `y + z` is the one destined for `x`. If the
// assignment were deferred to the code generator, the code generator
// would need the AST *and* a per-expression count of how many
// quadruples each expression produced — duplicating this module's
// logic and coupling two phases that are supposed to be independent.
//
// Note the division of labour with the symbol table (see
// `docs/09-codegen-implementation.md`):
//
//   - The SYMBOL TABLE says WHERE a variable lives (section + offset).
//   - The QUADRUPLE TABLE says WHAT operations the program performs.
//
// The `(=, T1, _, x)` quadruple names the destination `x`; it does not
// contain `x`'s address. The address is resolved later, from the symbol
// table. So the quadruple table is never used to *store* variable
// assignments — only to *describe* the assignment operation.
//
// Pattern: every `generate_*` method either:
//   (a) emits 0+ tuples and returns a `Value` representing the
//       expression's "result operand" for the caller; or
//   (b) handles control flow and returns the temporary that holds
//       the expression's value (control-flow expressions DO have a
//       value in Neko: `if` and `match` are expressions).
// ============================================================================

use super::emitter::CodeEmitter;

use crate::parser::ast::{
    AddOpChain, Arm, CmpOp, Expr, Literal, LogicalOpChain, MulOpChain, Pattern, Program, Stmt,
};

/// The "result" of evaluating an expression: either a literal/identifier
/// (no new temporary needed) or a fresh temporary that was just allocated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A plain operand: a literal (`90`, `"Hola"`) or a variable name (`x`).
    /// No quadruple was emitted; the operand is used directly.
    Operand(String),
}

impl Value {
    /// Convert to a string for use as an arg in a quadruple or triple.
    pub fn as_str(&self) -> String {
        match self {
            Value::Operand(s) => s.clone(),
        }
    }
}

pub struct ICGenerator {
    emitter: CodeEmitter,
}

impl ICGenerator {
    pub fn new() -> Self {
        Self {
            emitter: CodeEmitter::new(),
        }
    }

    /// Run the translation. Consumes the generator and returns both
    /// representations of the intermediate code.
    pub fn generate(mut self, program: &Program) -> GeneratedCode {
        for stmt in &program.statements {
            self.generate_stmt(stmt);
        }
        GeneratedCode {
            quadruples: self.emitter.render_quadruples(),
            triples: self.emitter.render_triples(),
            quad_count: self.emitter.quadruples().len(),
            triple_count: self.emitter.triples().len(),
            quadruple_list: self.emitter.quadruples().to_vec(),
            triple_list: self.emitter.triples().to_vec(),
        }
    }

    // ========================================================================
    // Statements
    // ========================================================================

    fn generate_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FnDecl { body } => {
                // The `fn neko {` wrapper does NOT emit any tuple.
                // Only the body does.
                for s in body {
                    self.generate_stmt(s);
                }
            }
            Stmt::VarDecl { name, value } => {
                // The DECLARATION part (`nyan name`) emits nothing — there
                // is no `DECLARE` quadruple and the keyword never appears
                // in the quadruple table.
                //
                // What we DO emit is the ASSIGNMENT of the value to the
                // variable, exactly like C's `a = 5;` generates code while
                // `int a;` does not. See the file header for the full
                // justification.
                let v = self.generate_expr(value);
                self.emitter.emit_quad("=", v.as_str(), "_", name.clone());
                self.emitter.emit_triple("=", v.as_str(), "_".to_string());
            }
            Stmt::PrintStmt { value } => {
                let v = self.generate_expr(value);
                self.emitter.emit_quad("PRINT", v.as_str(), "_", "_");
                self.emitter.emit_triple("PRINT", v.as_str(), "_");
            }
        }
    }

    // ========================================================================
    // Expressions
    //
    // Every `generate_expr` returns the Value where the expression's
    // result lives. The caller decides whether to wrap that in another
    // quadruple, ignore it, etc.
    // ========================================================================

    fn generate_expr(&mut self, expr: &Expr) -> Value {
        match expr {
            Expr::If { condition, then_branch, else_branch } => {
                self.generate_if(condition, then_branch, else_branch.as_deref())
            }
            Expr::Match { scrutinee, arms } => {
                self.generate_match(scrutinee, arms)
            }
            Expr::Or(c) => self.generate_or_chain(c),
            Expr::And(c) => self.generate_and_chain(c),
            Expr::Not { operand } => self.generate_not(operand),
            Expr::Compare { left, op, right } => {
                self.generate_compare(left, op.as_ref(), right.as_deref())
            }
            Expr::Add(c) => self.generate_add_chain(c),
            Expr::Mul(c) => self.generate_mul_chain(c),
            Expr::Negate { operand } => self.generate_negate(operand),
            Expr::Primary(lit) => self.generate_primary(lit),
        }
    }

    // ----- Primary -----

    fn generate_primary(&mut self, lit: &Literal) -> Value {
        match lit {
            Literal::Int(s) => Value::Operand(s.clone()),
            Literal::Str(s) => Value::Operand(s.clone()),
            Literal::True => Value::Operand("true".to_string()),
            Literal::False => Value::Operand("false".to_string()),
            Literal::Ident(name) => Value::Operand(name.clone()),
            Literal::Grouped(inner) => self.generate_expr(inner),
        }
    }

    // ----- Logical chains -----

    fn generate_or_chain(&mut self, chain: &crate::parser::ast::Chain<LogicalOpChain>) -> Value {
        // For a chain `a || b || c`:
        //   emit (||, a, b, T1)
        //   emit (||, T1, c, T2)
        //   return Value::Operand("T2")
        let mut acc = self.generate_expr(&chain.left);
        for (i, _op) in chain.ops.iter().enumerate() {
            let rhs = self.generate_expr(&chain.operands[i]);
            let t = self.emitter.next_temp();
            self.emitter.emit_quad("||", acc.as_str(), rhs.as_str(), t.clone());
            let tri_idx = self.emitter.emit_triple("||", acc.as_str(), rhs.as_str());
            // For triples, the next accumulator references this triple by index.
            // We do NOT modify `acc` here for triples; each triple is self-contained.
            // The accumulator for the next op is the temporary T_n.
            acc = Value::Operand(t);
            // Suppress unused warning for tri_idx (kept for debugging/trace).
            let _ = tri_idx;
        }
        acc
    }

    fn generate_and_chain(&mut self, chain: &crate::parser::ast::Chain<LogicalOpChain>) -> Value {
        let mut acc = self.generate_expr(&chain.left);
        for (i, _op) in chain.ops.iter().enumerate() {
            let rhs = self.generate_expr(&chain.operands[i]);
            let t = self.emitter.next_temp();
            self.emitter.emit_quad("&&", acc.as_str(), rhs.as_str(), t.clone());
            self.emitter.emit_triple("&&", acc.as_str(), rhs.as_str());
            acc = Value::Operand(t);
        }
        acc
    }

    fn generate_not(&mut self, operand: &Expr) -> Value {
        let v = self.generate_expr(operand);
        let t = self.emitter.next_temp();
        self.emitter.emit_quad("!", v.as_str(), "_", t.clone());
        self.emitter.emit_triple("!", v.as_str(), "_");
        Value::Operand(t)
    }

    // ----- Comparison -----

    fn generate_compare(
        &mut self,
        left: &Expr,
        op: Option<&CmpOp>,
        right: Option<&Expr>,
    ) -> Value {
        let l = self.generate_expr(left);
        if let (Some(op), Some(r)) = (op, right) {
            let op_str = match op {
                CmpOp::Eq => "==",
                CmpOp::Ne => "!=",
                CmpOp::Lt => "<",
                CmpOp::Gt => ">",
                CmpOp::Le => "<=",
                CmpOp::Ge => ">=",
            };
            let rhs = self.generate_expr(r);
            let t = self.emitter.next_temp();
            self.emitter.emit_quad(op_str, l.as_str(), rhs.as_str(), t.clone());
            self.emitter.emit_triple(op_str, l.as_str(), rhs.as_str());
            Value::Operand(t)
        } else {
            // No comparison — just pass through (e.g. when Compare wraps a
            // single value). This shouldn't normally happen if the
            // semantics allowed only one comparison, but we handle it.
            l
        }
    }

    // ----- Arithmetic chains -----

    fn generate_add_chain(&mut self, chain: &crate::parser::ast::Chain<AddOpChain>) -> Value {
        let mut acc = self.generate_expr(&chain.left);
        for (i, op) in chain.ops.iter().enumerate() {
            let op_str = match op {
                AddOpChain::Plus => "+",
                AddOpChain::Minus => "-",
            };
            let rhs = self.generate_expr(&chain.operands[i]);
            let t = self.emitter.next_temp();
            self.emitter.emit_quad(op_str, acc.as_str(), rhs.as_str(), t.clone());
            self.emitter.emit_triple(op_str, acc.as_str(), rhs.as_str());
            acc = Value::Operand(t);
        }
        acc
    }

    fn generate_mul_chain(&mut self, chain: &crate::parser::ast::Chain<MulOpChain>) -> Value {
        let mut acc = self.generate_expr(&chain.left);
        for (i, op) in chain.ops.iter().enumerate() {
            let op_str = match op {
                MulOpChain::Times => "*",
                MulOpChain::Div => "/",
                MulOpChain::Mod => "%",
            };
            let rhs = self.generate_expr(&chain.operands[i]);
            let t = self.emitter.next_temp();
            self.emitter.emit_quad(op_str, acc.as_str(), rhs.as_str(), t.clone());
            self.emitter.emit_triple(op_str, acc.as_str(), rhs.as_str());
            acc = Value::Operand(t);
        }
        acc
    }

    fn generate_negate(&mut self, operand: &Expr) -> Value {
        let v = self.generate_expr(operand);
        let t = self.emitter.next_temp();
        self.emitter.emit_quad("-", v.as_str(), "_", t.clone());
        self.emitter.emit_triple("-", v.as_str(), "_");
        Value::Operand(t)
    }

    // ----- Control flow -----

    /// `if <cond> <then> else <else>` — emits:
    ///   GOTOF cond L_else
    ///   <then>
    ///   GOTO L_end
    /// L_else:
    ///   <else>
    /// L_end:
    ///
    /// Because `if` is an EXPRESSION in Neko (it produces a value), the
    /// generator reserves a result temporary and assigns the value of
    /// whichever branch ran into it. The temporary is returned to the
    /// caller so it can be used (or assigned to a variable).
    ///
    /// With an else branch:
    ///   (GOTOF, cond, _, L_else)
    ///   <then>
    ///   (=, <then value>, _, T_result)
    ///   (GOTO, _, _, L_end)
    ///   (LABEL, _, _, L_else)
    ///   <else>
    ///   (=, <else value>, _, T_result)
    ///   (LABEL, _, _, L_end)
    fn generate_if(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: Option<&Expr>,
    ) -> Value {
        let cond = self.generate_expr(condition);

        // The temporary that will hold the value of the whole `if`.
        let result = self.emitter.next_temp();

        // Only emit GOTOF if there IS an else branch. If there is no
        // else, we treat the `if` as a conditional statement and emit
        // GOTOF then the then-branch, then a label.
        let l_else = self.emitter.next_label();
        let l_end = self.emitter.next_label();

        if let Some(_else) = else_branch {
            // (GOTOF, cond, _, L_else)
            self.emitter.emit_quad("GOTOF", cond.as_str(), "_", l_else.clone());
            self.emitter.emit_triple("GOTOF", cond.as_str(), "_".to_string());
            // then branch
            let then_value = self.generate_expr(then_branch);
            // (=, <then value>, _, T_result)
            self.emitter
                .emit_quad("=", then_value.as_str(), "_", result.clone());
            self.emitter
                .emit_triple("=", then_value.as_str(), "_".to_string());
            // (GOTO, _, _, L_end)
            self.emitter.emit_quad("GOTO", "_", "_", l_end.clone());
            self.emitter.emit_triple("GOTO", "_".to_string(), "_".to_string());
            // L_else:
            self.emitter.emit_quad("LABEL", "_", "_", l_else.clone());
            self.emitter.emit_triple("LABEL", "_".to_string(), "_".to_string());
            // else branch
            let else_value = self.generate_expr(_else);
            // (=, <else value>, _, T_result)
            self.emitter
                .emit_quad("=", else_value.as_str(), "_", result.clone());
            self.emitter
                .emit_triple("=", else_value.as_str(), "_".to_string());
            // L_end:
            self.emitter.emit_quad("LABEL", "_", "_", l_end.clone());
            self.emitter.emit_triple("LABEL", "_".to_string(), "_".to_string());
        } else {
            // No else branch: the `if` has no value on the false path, so
            // we pre-initialise the result temporary to 0 (a null pointer
            // for strings, zero for ints/bools) before the branch.
            self.emitter.emit_quad("=", "0", "_", result.clone());
            self.emitter.emit_triple("=", "0".to_string(), "_".to_string());
            // (GOTOF, cond, _, L_end)
            self.emitter.emit_quad("GOTOF", cond.as_str(), "_", l_end.clone());
            self.emitter.emit_triple("GOTOF", cond.as_str(), "_".to_string());
            // then branch
            let then_value = self.generate_expr(then_branch);
            // (=, <then value>, _, T_result)
            self.emitter
                .emit_quad("=", then_value.as_str(), "_", result.clone());
            self.emitter
                .emit_triple("=", then_value.as_str(), "_".to_string());
            // L_end:
            self.emitter.emit_quad("LABEL", "_", "_", l_end.clone());
            self.emitter.emit_triple("LABEL", "_".to_string(), "_".to_string());
        }

        Value::Operand(result)
    }

    /// `match <scrutinee> { <arms> }` — emits:
    /// For each arm i:
    ///   (==, scrut, pat_i, T_i)
    ///   (GOTOV, T_i, _, L_arm_i)
    /// L_default: <default body>          (only one default)
    /// L_arm_0: <body_0>
    ///   GOTO L_end
    /// L_arm_1: <body_1>
    ///   GOTO L_end
    /// ...
    /// L_end:
    fn generate_match(&mut self, scrutinee: &Expr, arms: &[Arm]) -> Value {
        let scrut = self.generate_expr(scrutinee);

        // The temporary that holds the value of the whole `match`.
        // The match is an expression in Neko, so it must produce a value.
        let result = self.emitter.next_temp();

        // Allocate all the comparison temporaries and arm labels up front
        // so we can use them in GOTOV instructions before the bodies.
        let l_end = self.emitter.next_label();

        // First pass: emit the condition tests for each arm in source order.
        // We need to remember each arm's body to emit AFTER all tests.
        // To keep things simple, we use a small struct.

        struct ArmInfo {
            label: String,
            body_value: Option<Value>, // filled in second pass
        }

        let mut arm_infos: Vec<ArmInfo> = Vec::new();
        let mut default_idx: Option<usize> = None;

        for arm in arms {
            let l_arm = self.emitter.next_label();
            let is_default = matches!(arm.pattern, Pattern::Wildcard);
            if is_default {
                default_idx = Some(arm_infos.len());
            }
            arm_infos.push(ArmInfo {
                label: l_arm,
                body_value: None,
            });
        }

        // Emit the condition tests for the non-default arms.
        //
        // The default arm gets NO test of its own. Instead, after all
        // the pattern tests, an unconditional GOTO jumps to it: that
        // jump is the "else" of the if-chain. Without it, control would
        // fall through into the FIRST arm's body, which would be wrong.
        //
        // The default arm's body is emitted LAST (see the second pass
        // below), so it falls through to L_end naturally and does not
        // need a GOTO of its own.
        for (i, arm) in arms.iter().enumerate() {
            if matches!(arm.pattern, Pattern::Wildcard) {
                // The default arm is handled after the loop.
                continue;
            }
            match &arm.pattern {
                Pattern::Int(s) => {
                    let t = self.emitter.next_temp();
                    self.emitter.emit_quad("==", scrut.as_str(), s.clone(), t.clone());
                    self.emitter.emit_triple("==", scrut.as_str(), s.clone());
                    self.emitter.emit_quad(
                        "GOTOV",
                        t.clone(),
                        "_",
                        arm_infos[i].label.clone(),
                    );
                    self.emitter.emit_triple("GOTOV", t, "_".to_string());
                }
                Pattern::Str(s) => {
                    let t = self.emitter.next_temp();
                    self.emitter.emit_quad("==", scrut.as_str(), s.clone(), t.clone());
                    self.emitter.emit_triple("==", scrut.as_str(), s.clone());
                    self.emitter.emit_quad(
                        "GOTOV",
                        t.clone(),
                        "_",
                        arm_infos[i].label.clone(),
                    );
                    self.emitter.emit_triple("GOTOV", t, "_".to_string());
                }
                Pattern::True => {
                    let t = self.emitter.next_temp();
                    self.emitter.emit_quad("==", scrut.as_str(), "true", t.clone());
                    self.emitter.emit_triple("==", scrut.as_str(), "true".to_string());
                    self.emitter
                        .emit_quad("GOTOV", t.clone(), "_", arm_infos[i].label.clone());
                    self.emitter.emit_triple("GOTOV", t, "_".to_string());
                }
                Pattern::False => {
                    let t = self.emitter.next_temp();
                    self.emitter.emit_quad("==", scrut.as_str(), "false", t.clone());
                    self.emitter.emit_triple("==", scrut.as_str(), "false".to_string());
                    self.emitter
                        .emit_quad("GOTOV", t.clone(), "_", arm_infos[i].label.clone());
                    self.emitter.emit_triple("GOTOV", t, "_".to_string());
                }
                Pattern::Bind(name) => {
                    let t = self.emitter.next_temp();
                    self.emitter.emit_quad("==", scrut.as_str(), name.clone(), t.clone());
                    self.emitter.emit_triple("==", scrut.as_str(), name.clone());
                    self.emitter
                        .emit_quad("GOTOV", t.clone(), "_", arm_infos[i].label.clone());
                    self.emitter.emit_triple("GOTOV", t, "_".to_string());
                }
                Pattern::Wildcard => {
                    // Unreachable: handled by the `continue` above.
                    unreachable!();
                }
            }
        }

        // The "else" of the if-chain: if none of the pattern tests
        // matched, jump to the default arm's body.
        if let Some(di) = default_idx {
            self.emitter
                .emit_quad("GOTO", "_", "_", arm_infos[di].label.clone());
            self.emitter.emit_triple("GOTO", "_".to_string(), "_".to_string());
        }

        // Second pass: emit each arm's body in source order, then GOTO L_end.
        // Default arm is the LAST one we emit (so any unmatched cases fall through).
        // Reorder: non-default first, default last.
        let mut order: Vec<usize> = (0..arms.len()).collect();
        if let Some(di) = default_idx {
            order.retain(|&i| i != di);
            order.push(di);
        }

        for (idx, i) in order.iter().enumerate() {
            self.emitter
                .emit_quad("LABEL", "_", "_", arm_infos[*i].label.clone());
            self.emitter.emit_triple("LABEL", "_".to_string(), "_".to_string());
            let body = self.generate_expr(&arms[*i].body);
            arm_infos[*i].body_value = Some(body.clone());
            // The match is an EXPRESSION: the value of the arm that ran
            // must land in the match's result temporary.
            self.emitter
                .emit_quad("=", body.as_str(), "_", result.clone());
            self.emitter.emit_triple("=", body.as_str(), "_".to_string());
            // After every body EXCEPT the last one (which is the default
            // and naturally falls through to L_end), emit a GOTO L_end
            // so the next arm's body is skipped.
            if idx + 1 < order.len() {
                self.emitter.emit_quad("GOTO", "_", "_", l_end.clone());
                self.emitter.emit_triple("GOTO", "_".to_string(), "_".to_string());
            }
        }

        // L_end:
        self.emitter.emit_quad("LABEL", "_", "_", l_end);
        self.emitter.emit_triple("LABEL", "_".to_string(), "_".to_string());

        Value::Operand(result)
    }
}

/// What the generator produced.
///
/// `quadruples` / `triples` hold the RENDERED text (what goes into
/// `output/quadruples.txt` and `output/triples.txt`).
///
/// `quadruple_list` / `triple_list` hold the STRUCTURED tuples, which the
/// code generator (`src/codegen/`) consumes to emit assembly. Keeping both
/// means the code generator does not have to re-parse the rendered text.
pub struct GeneratedCode {
    pub quadruples: String,
    pub triples: String,
    pub quad_count: usize,
    pub triple_count: usize,
    /// The quadruples as structured data, for the assembly generator.
    pub quadruple_list: Vec<super::quadruple::Quadruple>,
    /// The triples as structured data. Kept for symmetry with
    /// `quadruple_list` and for any future consumer; the assembly
    /// generator works from the quadruples.
    #[allow(dead_code)]
    pub triple_list: Vec<super::triple::Triple>,
}
