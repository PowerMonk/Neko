// ============================================================================
// Neko Semantic Analyzer
//
// What this does
// ---------------
// Consumes a `Program` AST (from the parser) plus the token stream that
// produced it, and performs the four static checks defined in
// `docs/06-semantic-conventions.md`:
//
//   1. Comprobación de tipos         → type-check every expression
//   2. Flujo de control              → exhaustividad de match
//   3. Unicidad                      → no redeclaración, no shadowing,
//                                      no duplicate match arm
//   4. Nombres                       → (no aplica en esta iteración,
//                                      categoría reservada)
//
// Plus dos comprobaciones auxiliares documentadas en §5 del doc:
//   - Identificador indefinido / uso antes de declaración
//   - Variable declarada pero no usada
//
// How it works
// ------------
// Visitor over the AST. Each `visit_*` function returns the inferred
// `Type` of the node (or `None` for statements, which don't produce a
// value). Errors are accumulated into `self.errors` — collect-don't-bail,
// same philosophy as the parser.
//
// Source positions
// ----------------
// The AST doesn't carry line/column on every node. To report errors at
// the right place, we walk `tokens` in lockstep with the AST. Each
// `visit_*` consumes the tokens it expects, and `self.current_pos()`
// reads the line/column of the next unconsumed token.
// ============================================================================

use super::error::SemanticError;
use super::scope::{Scope, Symbol};
use super::types::Type;

use crate::lexer::{Token, TokenKind};
use crate::parser::ast::{
    AddOpChain, Arm, Chain, CmpOp, Expr, Literal, LogicalOpChain, MulOpChain, Pattern, Program,
    Stmt,
};

/// Final result of the semantic analysis pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticAnalysis {
    pub errors: Vec<SemanticError>,
}

impl SemanticAnalysis {
    pub fn render_errors(&self) -> String {
        let mut out = String::new();
        for e in &self.errors {
            out.push_str(&e.format_line());
            out.push('\n');
        }
        out
    }

    pub fn write_output(&self, dir: &str) -> std::io::Result<()> {
        let path = std::path::Path::new(dir);
        std::fs::create_dir_all(path)?;
        std::fs::write(path.join("semantic_errors.txt"), self.render_errors())?;
        Ok(())
    }
}

/// One declared name and its inferred type. Used to detect unused
/// variables after the visit pass is done.
#[derive(Debug, Clone)]
struct Declared {
    name: String,
}

/// The semantic analyzer. One instance per run.
pub struct SemanticAnalyzer {
    /// Tokens of the program, walked in lockstep with the AST for
    /// position information. See the file header for why.
    tokens: Vec<Token>,
    /// Cursor into `tokens`. Points to the NEXT token to consume.
    cursor: usize,
    /// Collected errors (collect-don't-bail).
    errors: Vec<SemanticError>,
    /// Current scope (top of the chain).
    scope: Scope,
    /// Names declared during the visit pass. After the visit, any name
    /// here that wasn't read is reported as "unused variable".
    declared: Vec<Declared>,
    /// How many times each name was read during the visit.
    read_count: std::collections::HashMap<String, usize>,
}

impl SemanticAnalyzer {
    pub fn new(tokens: Vec<Token>) -> Self {
        // Filter out COMMENT tokens (whitespace-like from the parser's
        // point of view). The lexer emits them for convenience but
        // neither parser nor semantic analyzer should see them.
        let filtered: Vec<Token> = tokens
            .into_iter()
            .filter(|t| t.kind != TokenKind::Comment)
            .collect();
        Self {
            tokens: filtered,
            cursor: 0,
            errors: Vec::new(),
            scope: Scope::new_root(),
            declared: Vec::new(),
            read_count: std::collections::HashMap::new(),
        }
    }

    /// Run the analysis. Consumes the analyzer (no re-use).
    pub fn analyze(mut self, program: &Program) -> SemanticAnalysis {
        // ---- Pass 1: walk statements in order, inferring types and
        //              reporting use-before-declaration errors. ----
        for stmt in &program.statements {
            self.visit_stmt(stmt);
        }

        // ---- Pass 2: detect unused variables ----
        for d in &self.declared {
            let used = self.read_count.get(&d.name).copied().unwrap_or(0);
            if used == 0 {
                let (line, col) = self.last_consumed_position();
                self.errors.push(SemanticError::new(
                    line,
                    col,
                    format!("unused variable: '{}'", d.name),
                ));
            }
        }

        SemanticAnalysis {
            errors: self.errors,
        }
    }

    // ========================================================================
    // Token cursor helpers
    // ========================================================================

    /// Look at the next token without consuming it.
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    /// Peek the kind of the next token.
    fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|t| t.kind)
    }

    /// Consume the next token if it matches `kind`.
    fn consume(&mut self, kind: TokenKind) -> Option<Token> {
        if self.peek_kind() == Some(kind) {
            let t = self.tokens[self.cursor].clone();
            self.cursor += 1;
            Some(t)
        } else {
            None
        }
    }

    /// Consume the next token unconditionally (used for primary
    /// expression tokens whose kind we already know from the AST).
    fn consume_any(&mut self) -> Option<Token> {
        if self.cursor < self.tokens.len() {
            let t = self.tokens[self.cursor].clone();
            self.cursor += 1;
            Some(t)
        } else {
            None
        }
    }

    /// Current position (line, column) of the next unconsumed token, or
    /// (0, 0) at EOF.
    fn current_pos(&self) -> (usize, usize) {
        self.peek()
            .map(|t| (t.line, t.column))
            .unwrap_or((0, 0))
    }

    /// Position of the most recently consumed token (or (0, 0) if none).
    fn last_consumed_position(&self) -> (usize, usize) {
        if self.cursor == 0 {
            (0, 0)
        } else {
            let t = &self.tokens[self.cursor - 1];
            (t.line, t.column)
        }
    }

    // ========================================================================
    // Error helpers
    // ========================================================================

    fn error(&mut self, message: impl Into<String>) {
        let (line, col) = self.current_pos();
        self.errors.push(SemanticError::new(line, col, message));
    }

    fn error_at(&mut self, token: &Token, message: impl Into<String>) {
        self.errors
            .push(SemanticError::new(token.line, token.column, message));
    }

    // ========================================================================
    // Scope helpers
    // ========================================================================

    /// Open a new scope on top of the current one.
    fn push_scope(&mut self) {
        let new_scope = self.scope.child();
        self.scope = new_scope;
    }

    /// Close the current scope and return to its parent. Returns `false`
    /// if the analyzer was already at the root scope.
    fn pop_scope(&mut self) -> bool {
        // Replace `self.scope` with its parent. We swap-then-take so we
        // don't have to clone the chain.
        let mut current = Scope::new_root();
        std::mem::swap(&mut current, &mut self.scope);
        // Peek the parent by inspecting the current scope before
        // moving it. We use `parent_ref` (borrowed) instead of
        // `into_parent` (consuming) so we can restore on failure.
        let parent_opt: Option<Scope> = current.parent_ref().cloned();
        if let Some(parent) = parent_opt {
            self.scope = parent;
            true
        } else {
            // Already at root — restore current.
            self.scope = current;
            false
        }
    }

    // ========================================================================
    // Statements
    // ========================================================================

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FnDecl { body } => self.visit_fn_decl(body),
            Stmt::VarDecl { name, value } => self.visit_var_decl(name, value),
            Stmt::PrintStmt { value } => self.visit_print_stmt(value),
        }
    }

    /// `fn neko { <stmts> }` — consume `fn`, `neko`, `{`, walk body,
    /// consume `}`.
    fn visit_fn_decl(&mut self, body: &Vec<Stmt>) {
        self.consume(TokenKind::KeywordFn);
        self.consume(TokenKind::KeywordNeko);
        self.consume(TokenKind::LeftBrace);

        self.push_scope();
        for s in body {
            self.visit_stmt(s);
        }
        self.pop_scope();

        self.consume(TokenKind::RightBrace);
    }

    /// `nyan <id> = <expr>` — type-check the expression, then declare
    /// `id` with that inferred type. Performs checks 3 (unicidad) and 1
    /// (scope/sin shadowing).
    fn visit_var_decl(&mut self, name: &str, value: &Expr) {
        self.consume(TokenKind::KeywordNyan);

        // The identifier token: must be present.
        let id_token = match self.consume(TokenKind::Identifier) {
            Some(t) => t,
            None => {
                self.error(format!(
                    "expected variable name, found '{}'",
                    self.peek().map(|t| t.lexeme.as_str()).unwrap_or("EOF")
                ));
                self.consume_any();
                return;
            }
        };

        self.consume(TokenKind::Assign);

        // Type-check the initializer expression.
        let value_type = self.visit_expr(value);

        // Uniqueness + no-shadowing.
        if self.scope.contains_local(name) {
            // Already in current scope — redeclaration.
            self.error_at(
                &id_token,
                format!("redeclaration of '{}' in the same scope", name),
            );
        } else if self.scope.contains_in_chain(name) {
            // In a parent scope — shadowing (Neko disallows).
            self.error_at(
                &id_token,
                format!(
                    "'{}' shadows an outer declaration (Neko disallows shadowing)",
                    name
                ),
            );
        } else {
            // Fresh name — insert.
            if self
                .scope
                .insert_local(name, Symbol::new(name, value_type))
                .is_err()
            {
                // Should not happen because contains_local returned
                // false; defensive only.
                self.error_at(
                    &id_token,
                    format!("redeclaration of '{}' in the same scope", name),
                );
            } else {
                self.declared.push(Declared {
                    name: name.to_string(),
                });
            }
        }
    }

    /// `meow(<expr>)` — type-check the expression, no further checks.
    fn visit_print_stmt(&mut self, value: &Expr) {
        self.consume(TokenKind::KeywordMeow);
        self.consume(TokenKind::LeftParen);
        let _ = self.visit_expr(value);
        self.consume(TokenKind::RightParen);
    }

    // ========================================================================
    // Expressions — return the inferred type
    // ========================================================================

    fn visit_expr(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => self.visit_if(condition, then_branch, else_branch.as_deref()),
            Expr::Match { scrutinee, arms } => self.visit_match(scrutinee, arms),
            Expr::Or(c) => self.visit_or_chain(c),
            Expr::And(c) => self.visit_and_chain(c),
            Expr::Not { operand } => self.visit_not(operand),
            Expr::Compare { left, op, right } => {
                self.visit_compare(left, op.as_ref(), right.as_deref())
            }
            Expr::Add(c) => self.visit_add_chain(c),
            Expr::Mul(c) => self.visit_mul_chain(c),
            Expr::Negate { operand } => self.visit_negate(operand),
            Expr::Primary(lit) => self.visit_primary(lit),
        }
    }

    /// `if <cond> <then> else <else>`.
    fn visit_if(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: Option<&Expr>,
    ) -> Type {
        self.consume(TokenKind::KeywordIf);

        let cond_ty = self.visit_expr(condition);
        if cond_ty != Type::Bool {
            self.error(format!(
                "type mismatch: condition of 'if' must be bool, found {}",
                cond_ty.name()
            ));
        }

        self.consume(TokenKind::LeftBrace);
        self.push_scope();
        let then_ty = self.visit_expr(then_branch);
        self.pop_scope();
        self.consume(TokenKind::RightBrace);

        let else_ty = match else_branch {
            Some(e) => {
                self.consume(TokenKind::KeywordElse);
                self.consume(TokenKind::LeftBrace);
                self.push_scope();
                let t = self.visit_expr(e);
                self.pop_scope();
                self.consume(TokenKind::RightBrace);
                t
            }
            None => Type::Int, // no else: type unconstrained in this iteration
        };

        if let Some(_) = else_branch {
            if then_ty != else_ty {
                self.error(format!(
                    "type mismatch: branches of 'if' return different types ({} vs {})",
                    then_ty.name(),
                    else_ty.name()
                ));
            }
        }

        then_ty
    }

    /// `match <scrutinee> { <arms> }`.
    /// Performs comprobación 2 (exhaustividad) + 3 (unicidad de patrón)
    /// + 1 (tipo consistente entre brazos).
    fn visit_match(&mut self, scrutinee: &Expr, arms: &[Arm]) -> Type {
        self.consume(TokenKind::KeywordMatch);

        let scrutinee_ty = self.visit_expr(scrutinee);
        self.consume(TokenKind::LeftBrace);

        let mut seen_patterns: Vec<String> = Vec::new();
        let mut has_default = false;
        let mut arm_types: Vec<Type> = Vec::new();

        for arm in arms {
            let pattern_key = self.visit_pattern(&arm.pattern);
            match &pattern_key {
                PatternKey::Default => has_default = true,
                PatternKey::Literal(s) => {
                    if seen_patterns.iter().any(|p| p == s) {
                        self.error(
                            "duplicate match arm: pattern already used in a previous arm"
                                .to_string(),
                        );
                    } else {
                        seen_patterns.push(s.clone());
                    }
                }
            }
            self.consume(TokenKind::FatArrow);
            let arm_ty = self.visit_expr(&arm.body);
            arm_types.push(arm_ty);
        }

        if !has_default {
            self.error("non-exhaustive match: default arm ('_') is required");
        }

        if !arm_types.is_empty() {
            let first = arm_types[0];
            for t in &arm_types[1..] {
                if *t != first {
                    self.error(format!(
                        "type mismatch: arms of 'match' return different types ({} vs {})",
                        first.name(),
                        t.name()
                    ));
                    break;
                }
            }
        }

        self.consume(TokenKind::RightBrace);

        let _ = scrutinee_ty;
        arm_types.first().copied().unwrap_or(Type::Int)
    }

    /// Consume a pattern's tokens and return a key for duplicate detection.
    fn visit_pattern(&mut self, p: &Pattern) -> PatternKey {
        match p {
            Pattern::Wildcard => {
                self.consume(TokenKind::Wildcard);
                PatternKey::Default
            }
            Pattern::Int(s) => {
                self.consume(TokenKind::Integer);
                PatternKey::Literal(s.clone())
            }
            Pattern::Str(s) => {
                self.consume(TokenKind::StringLiteral);
                PatternKey::Literal(s.clone())
            }
            Pattern::Bind(name) => {
                self.consume(TokenKind::Identifier);
                PatternKey::Literal(name.clone())
            }
            Pattern::True => {
                self.consume(TokenKind::KeywordTrue);
                PatternKey::Literal("true".into())
            }
            Pattern::False => {
                self.consume(TokenKind::KeywordFalse);
                PatternKey::Literal("false".into())
            }
        }
    }

    // ---- Logical chains ----

    fn visit_or_chain(&mut self, chain: &Chain<LogicalOpChain>) -> Type {
        self.visit_logical_chain(chain, |op| match op {
            LogicalOpChain::Or => TokenKind::OrOr,
            LogicalOpChain::And => TokenKind::AndAnd,
        })
    }

    fn visit_and_chain(&mut self, chain: &Chain<LogicalOpChain>) -> Type {
        self.visit_logical_chain(chain, |op| match op {
            LogicalOpChain::Or => TokenKind::OrOr,
            LogicalOpChain::And => TokenKind::AndAnd,
        })
    }

    fn visit_logical_chain<F>(&mut self, chain: &Chain<LogicalOpChain>, op_kind: F) -> Type
    where
        F: Fn(&LogicalOpChain) -> TokenKind,
    {
        let mut ty = self.visit_expr(&chain.left);
        for (i, op) in chain.ops.iter().enumerate() {
            self.consume(op_kind(op));
            let rhs_ty = self.visit_expr(&chain.operands[i]);
            if ty != Type::Bool || rhs_ty != Type::Bool {
                self.error(format!(
                    "type mismatch: logical operator requires bool operands, found {} and {}",
                    ty.name(),
                    rhs_ty.name()
                ));
            }
            ty = Type::Bool;
        }
        ty
    }

    /// `!x` — operand must be `bool`; result is `bool`.
    fn visit_not(&mut self, operand: &Expr) -> Type {
        self.consume(TokenKind::Bang);
        let ty = self.visit_expr(operand);
        if ty != Type::Bool {
            self.error(format!(
                "cannot apply '!' to non-bool value of type {}",
                ty.name()
            ));
        }
        Type::Bool
    }

    /// Comparison: both operands must be the same type; result is `bool`.
    fn visit_compare(
        &mut self,
        left: &Expr,
        op: Option<&CmpOp>,
        right: Option<&Expr>,
    ) -> Type {
        let lty = self.visit_expr(left);
        if let (Some(op), Some(r)) = (op, right) {
            let kind = match op {
                CmpOp::Eq => TokenKind::EqualEqual,
                CmpOp::Ne => TokenKind::NotEqual,
                CmpOp::Lt => TokenKind::Less,
                CmpOp::Gt => TokenKind::Greater,
                CmpOp::Le => TokenKind::LessEqual,
                CmpOp::Ge => TokenKind::GreaterEqual,
            };
            self.consume(kind);
            let rty = self.visit_expr(r);
            if lty != rty {
                self.error(format!(
                    "type mismatch: comparison operands must be the same type, found {} and {}",
                    lty.name(),
                    rty.name()
                ));
            }
            Type::Bool
        } else {
            lty
        }
    }

    // ---- Arithmetic chains ----

    fn visit_add_chain(&mut self, chain: &Chain<AddOpChain>) -> Type {
        self.visit_arith_chain(chain, |op| match op {
            AddOpChain::Plus => TokenKind::Plus,
            AddOpChain::Minus => TokenKind::Minus,
        })
    }

    fn visit_mul_chain(&mut self, chain: &Chain<MulOpChain>) -> Type {
        self.visit_arith_chain(chain, |op| match op {
            MulOpChain::Times => TokenKind::Star,
            MulOpChain::Div => TokenKind::Slash,
            MulOpChain::Mod => TokenKind::Percent,
        })
    }

    fn visit_arith_chain<Op, F>(&mut self, chain: &Chain<Op>, op_kind: F) -> Type
    where
        F: Fn(&Op) -> TokenKind,
    {
        let mut ty = self.visit_expr(&chain.left);
        for (i, op) in chain.ops.iter().enumerate() {
            self.consume(op_kind(op));
            let rhs_ty = self.visit_expr(&chain.operands[i]);
            if ty != Type::Int || rhs_ty != Type::Int {
                self.error(format!(
                    "type mismatch: arithmetic operator requires int operands, found {} and {}",
                    ty.name(),
                    rhs_ty.name()
                ));
            }
            ty = Type::Int;
        }
        ty
    }

    /// `-x` — operand must be `int`; result is `int`.
    fn visit_negate(&mut self, operand: &Expr) -> Type {
        self.consume(TokenKind::Minus);
        let ty = self.visit_expr(operand);
        if ty != Type::Int {
            self.error(format!(
                "cannot negate non-int value of type {}",
                ty.name()
            ));
        }
        Type::Int
    }

    // ---- Primary ----

    fn visit_primary(&mut self, lit: &Literal) -> Type {
        match lit {
            Literal::Int(_) => {
                self.consume(TokenKind::Integer);
                Type::Int
            }
            Literal::Str(_) => {
                self.consume(TokenKind::StringLiteral);
                Type::String
            }
            Literal::True => {
                self.consume(TokenKind::KeywordTrue);
                Type::Bool
            }
            Literal::False => {
                self.consume(TokenKind::KeywordFalse);
                Type::Bool
            }
            Literal::Ident(name) => self.visit_ident(name),
            Literal::Grouped(inner) => {
                self.consume(TokenKind::LeftParen);
                let t = self.visit_expr(inner);
                self.consume(TokenKind::RightParen);
                t
            }
        }
    }

    /// Resolve an identifier. If it's in scope, increment its read count
    /// and return its type. Otherwise emit "undefined identifier" and
    /// return `Int` so type-checking can continue.
    fn visit_ident(&mut self, name: &str) -> Type {
        let id_token = self.consume(TokenKind::Identifier);
        match self.scope.lookup(name) {
            Some(sym) => {
                *self.read_count.entry(name.to_string()).or_insert(0) += 1;
                sym.ty
            }
            None => {
                let msg = format!("undefined identifier: '{}'", name);
                match id_token {
                    Some(t) => self.error_at(&t, msg),
                    None => self.error(msg),
                }
                Type::Int
            }
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Key used for duplicate-pattern detection in `match`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PatternKey {
    Default,
    Literal(String),
}
