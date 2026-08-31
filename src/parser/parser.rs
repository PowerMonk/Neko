// ============================================================================
// Neko Parser
//
// Pipeline position: consumes the `Vec<Token>` produced by the lexer
// (src/lexer/) and produces a `Program` AST plus a list of syntax errors.
//
// Style: recursive-descent, one `parse_xxx` function per non-terminal in
// `docs/02-grammar.md`. The grammar is LL(1)-friendly: every choice can be
// made by looking at the next token. Error recovery is *collect-don't-bail*
// — the parser never panics on bad input; it appends to its error list
// and tries to advance past the problem so subsequent errors can still
// be reported (mirrors the lexer's philosophy).
//
// The AST representation (src/parser/ast.rs) flattens the left-recursive
// *Tail chains from the grammar into a single `Vec<OpKind>` paired with
// the operand at the same recursion depth. See the operator-ladder
// section below for the exact pairing rule.
//
// Parse tree vs AST
// -----------------
// The parse tree in `docs/04-parse-tree-example.mmd` shows the FULL
// descent: `score` alone becomes Expr → OrExpr → AndExpr → NotExpr →
// CmpExpr → AddExpr → MulExpr → Unary → Primary → ID. The AST does NOT
// carry that 8-level chain for a simple variable reference — it collapses
// everything below `Expr::Or { left, tail }` into a single node whose
// `left` is the leading operand and whose `tail` is a flat list of
// operators (the operands at positions [1..] are part of the same Vec).
// ============================================================================

use super::ast::{
    AddOpChain, Arm, Chain, CmpOp, Expr, Literal, LogicalOpChain, MulOpChain, Pattern, Program,
    Stmt,
};
use super::error::SyntaxError;
use super::token_stream::TokenStream;

use crate::lexer::{Token, TokenKind};

/// Final result of the syntactic analysis pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntacticAnalysis {
    pub program: Program,
    pub errors: Vec<SyntaxError>,
}

impl SyntacticAnalysis {
    /// Render the AST as an indented text tree. One line per node, depth
    /// indicated by leading spaces (2 per level). This is intentionally
    /// simpler than the Mermaid parse tree: the AST has already collapsed
    /// the *Tail chains, so it's much smaller.
    pub fn render_tree(&self) -> String {
        let mut out = String::new();
        out.push_str("Program\n");
        for stmt in &self.program.statements {
            render_stmt(&mut out, stmt, 1);
        }
        out
    }

    pub fn render_errors(&self) -> String {
        let mut out = String::new();
        for e in &self.errors {
            out.push_str(&e.format_line());
            out.push('\n');
        }
        out
    }

    pub fn write_outputs(&self, dir: &str) -> std::io::Result<()> {
        let path = std::path::Path::new(dir);
        std::fs::create_dir_all(path)?;
        std::fs::write(path.join("parse_tree.txt"), self.render_tree())?;
        std::fs::write(path.join("parse_errors.txt"), self.render_errors())?;
        Ok(())
    }
}

/// The parser. One instance per analysis run.
pub struct Parser {
    stream: TokenStream,
    errors: Vec<SyntaxError>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            stream: TokenStream::new(tokens),
            errors: Vec::new(),
        }
    }

    /// Run the full parse pipeline. Consumes the parser (no re-use).
    pub fn parse(mut self) -> SyntacticAnalysis {
        let program = self.parse_program();
        SyntacticAnalysis {
            program,
            errors: self.errors,
        }
    }

    // ========================================================================
    // Error helpers
    // ========================================================================

    /// Record an error at the current cursor position.
    fn record_error(&mut self, message: impl Into<String>) {
        let (line, col) = self
            .stream
            .peek()
            .map(|t| (t.line, t.column))
            .unwrap_or((0, 0));
        self.errors.push(SyntaxError::new(line, col, message));
    }

    /// Record an error pointing at a *specific* token (often the one we
    /// just consumed and want to flag retrospectively).
    fn record_error_at(&mut self, token: &Token, message: impl Into<String>) {
        self.errors
            .push(SyntaxError::new(token.line, token.column, message));
    }

    /// Is the current token one that *starts* a new statement?
    fn is_statement_start(&self) -> bool {
        matches!(
            self.stream.peek_kind(),
            Some(
                TokenKind::KeywordFn
                    | TokenKind::KeywordNyan
                    | TokenKind::KeywordMeow
                    | TokenKind::KeywordIf
                    | TokenKind::KeywordMatch
            )
        )
    }

    /// Is the current token one that can start a Pattern?
    fn is_pattern_start(&self) -> bool {
        matches!(
            self.stream.peek_kind(),
            Some(
                TokenKind::Wildcard
                    | TokenKind::Integer
                    | TokenKind::StringLiteral
                    | TokenKind::Identifier
                    | TokenKind::KeywordTrue
                    | TokenKind::KeywordFalse
            )
        )
    }

    // ========================================================================
    // Grammar entry: Program → Stmts
    // ========================================================================

    /// Grammar: `Program → Stmts`
    fn parse_program(&mut self) -> Program {
        Program {
            statements: self.parse_stmts(),
        }
    }

    /// Grammar: `Stmts → Stmt Stmts | ε`
    ///
    /// Iteration in the grammar maps to a `while` loop in code: keep
    /// collecting statements until we hit EOF or a `}` (block close).
    /// The `ε` case is just "fall out of the loop with an empty Vec".
    fn parse_stmts(&mut self) -> Vec<Stmt> {
        let mut out = Vec::new();
        while !self.stream.is_eof() {
            if matches!(self.stream.peek_kind(), Some(TokenKind::RightBrace)) {
                break;
            }
            if !self.is_statement_start() {
                let what = match self.stream.peek() {
                    Some(t) => format!("Unexpected token '{}'", t.lexeme),
                    None => "Unexpected end of input".to_string(),
                };
                self.record_error(what);
                self.stream.advance();
                continue;
            }
            out.push(self.parse_stmt());
        }
        out
    }

    // ========================================================================
    // Stmt dispatch: Stmt → FnDecl | VarDecl | PrintStmt
    // ========================================================================

    fn parse_stmt(&mut self) -> Stmt {
        match self.stream.peek_kind() {
            Some(TokenKind::KeywordFn) => self.parse_fn_decl(),
            Some(TokenKind::KeywordNyan) => self.parse_var_decl(),
            Some(TokenKind::KeywordMeow) => self.parse_print_stmt(),
            _ => {
                self.record_error("Expected statement start");
                // Make forward progress: consume the offending token so
                // the outer parse_stmts loop doesn't spin forever.
                self.stream.advance();
                Stmt::PrintStmt {
                    value: Box::new(Expr::Primary(Literal::Int("0".into()))),
                }
            }
        }
    }

    /// Grammar: `FnDecl → 'fn' 'neko' Block`
    fn parse_fn_decl(&mut self) -> Stmt {
        self.stream.advance(); // 'fn'
        self.stream.advance(); // 'neko'
        let body = self.parse_block();
        Stmt::FnDecl { body }
    }

    /// Grammar: `Block → '{' Stmts '}'` (function body)
    ///
    /// Note: a *function* block contains a sequence of statements. The
    /// `if`/`else` and `match` arms use `parse_block_expr` instead, which
    /// holds a single Expr (those are expression contexts, not statement
    /// contexts).
    fn parse_block(&mut self) -> Vec<Stmt> {
        if self.stream.consume(TokenKind::LeftBrace).is_none() {
            self.record_error("Expected '{' to start block");
        }
        let body = self.parse_stmts();
        if self.stream.consume(TokenKind::RightBrace).is_none() {
            self.record_error("Expected '}' to close block");
        }
        body
    }

    /// Used inside `if` and `else` and inside `match` arms: a block that
    /// holds exactly one Expr (the value of the arm).
    ///
    /// Note: the grammar in docs/02-grammar.md treats this as the same
    /// `Block` production, but for the AST we split it into two
    /// functions so the parser knows whether to expect statements or
    /// a single expression.
    fn parse_block_expr(&mut self) -> Box<Expr> {
        if self.stream.consume(TokenKind::LeftBrace).is_none() {
            self.record_error("Expected '{' to start block");
        }
        let body = self.parse_expr();
        if self.stream.consume(TokenKind::RightBrace).is_none() {
            self.record_error("Expected '}' to close block");
        }
        body
    }

    /// Grammar: `VarDecl → 'nyan' ID '=' Expr`
    fn parse_var_decl(&mut self) -> Stmt {
        self.stream.advance(); // 'nyan'

        let name_token = match self.stream.advance() {
            Some(t) if t.kind == TokenKind::Identifier => t,
            Some(t) => {
                self.record_error_at(
                    &t,
                    format!("Expected variable name, found '{}'", t.lexeme),
                );
                Token::new(TokenKind::Identifier, "?", t.line, t.column)
            }
            None => {
                self.record_error("Expected variable name, found end of input");
                Token::new(TokenKind::Identifier, "?", 0, 0)
            }
        };

        if self.stream.consume(TokenKind::Assign).is_none() {
            self.record_error(format!(
                "Expected '=' after variable name, found '{}'",
                self.stream
                    .peek()
                    .map(|t| t.lexeme.as_str())
                    .unwrap_or("EOF")
            ));
        }

        let value = self.parse_expr();

        Stmt::VarDecl {
            name: name_token.lexeme,
            value,
        }
    }

    /// Grammar: `PrintStmt → 'meow' '(' Expr ')'`
    fn parse_print_stmt(&mut self) -> Stmt {
        self.stream.advance(); // 'meow'

        if self.stream.consume(TokenKind::LeftParen).is_none() {
            self.record_error("Expected '(' after 'meow'");
        }

        let value = self.parse_expr();

        if self.stream.consume(TokenKind::RightParen).is_none() {
            self.record_error("Expected ')' after expression");
        }

        Stmt::PrintStmt { value }
    }

    // ========================================================================
    // Expr dispatch
    //
    // Grammar: `Expr → 'if' Expr Block ElsePart
    //                | 'match' Expr '{' Arms '}'
    //                | OrExpr`
    // ========================================================================

    fn parse_expr(&mut self) -> Box<Expr> {
        match self.stream.peek_kind() {
            Some(TokenKind::KeywordIf) => Box::new(self.parse_if_expr()),
            Some(TokenKind::KeywordMatch) => Box::new(self.parse_match_expr()),
            _ => Box::new(self.parse_or_expr()),
        }
    }

    /// `Expr → 'if' Expr Block ElsePart`
    fn parse_if_expr(&mut self) -> Expr {
        self.stream.advance(); // 'if'

        let condition = self.parse_expr();
        let then_branch = self.parse_block_expr();
        let else_branch = self.parse_else_part();

        Expr::If {
            condition,
            then_branch,
            else_branch,
        }
    }

    /// `ElsePart → 'else' Block | ε`
    fn parse_else_part(&mut self) -> Option<Box<Expr>> {
        if self.stream.consume(TokenKind::KeywordElse).is_some() {
            Some(self.parse_block_expr())
        } else {
            None
        }
    }

    /// `Expr → 'match' Expr '{' Arms '}'`
    fn parse_match_expr(&mut self) -> Expr {
        self.stream.advance(); // 'match'

        let scrutinee = self.parse_expr();

        if self.stream.consume(TokenKind::LeftBrace).is_none() {
            self.record_error("Expected '{' to start match body");
        }

        let arms = self.parse_arms();

        if self.stream.consume(TokenKind::RightBrace).is_none() {
            self.record_error("Expected '}' to close match body");
        }

        Expr::Match { scrutinee, arms }
    }

    /// `Arms → Arm Arms | ε`
    fn parse_arms(&mut self) -> Vec<Arm> {
        let mut arms = Vec::new();
        while self.is_pattern_start() {
            arms.push(self.parse_arm());
        }
        arms
    }

    /// `Arm → Pattern '=>' Expr`
    fn parse_arm(&mut self) -> Arm {
        let pattern = self.parse_pattern();
        if self.stream.consume(TokenKind::FatArrow).is_none() {
            self.record_error("Expected '=>' in match arm");
        }
        let body = self.parse_expr();
        Arm { pattern, body }
    }

    /// `Pattern → '_' | INT | STRING | ID | 'true' | 'false'`
    fn parse_pattern(&mut self) -> Pattern {
        let token = match self.stream.advance() {
            Some(t) => t,
            None => {
                self.record_error("Expected pattern, found end of input");
                return Pattern::Wildcard;
            }
        };
        match token.kind {
            TokenKind::Wildcard => Pattern::Wildcard,
            TokenKind::Integer => Pattern::Int(token.lexeme),
            TokenKind::StringLiteral => Pattern::Str(token.lexeme),
            TokenKind::Identifier => Pattern::Bind(token.lexeme),
            TokenKind::KeywordTrue => Pattern::True,
            TokenKind::KeywordFalse => Pattern::False,
            _ => {
                self.record_error_at(&token, format!("Invalid pattern '{}'", token.lexeme));
                Pattern::Wildcard
            }
        }
    }

    // ========================================================================
    // Operator-precedence ladder (left-recursive in the grammar, flattened
    // in the AST).
    //
    // Pairing rule for the flattened tail vectors
    // -------------------------------------------
    // Each level stores its tail as `Vec<OpKind>` — only the operator
    // kinds, NOT the operands. The operands are already in the AST as
    // siblings: the head function consumes ONE operand for the `left`
    // field, then loops consuming (operator, operand) pairs and stuffing
    // both into the node:
    //
    //     left ← first operand
    //     tail.push(op_1); operand_1 = parse_*(...)
    //     tail.push(op_2); operand_2 = parse_*(...)
    //
    // So the AST representation is `Vec<OpKind>` PLUS the leading
    // `left` Expr, and the operands at tail[i] are stored in a parallel
    // Vec of Expr that lives next to the operators. That's why each
    // chain node carries BOTH `tail: Vec<OpKind>` AND `operands:
    // Vec<Expr>` — one for the operators, one for the operands after
    // the leading left.
    //
    // Wait — let me reconsider the AST design. The current `Expr::Add`
    // has only `{ left, tail: Vec<AddOpChain> }` and no operands Vec.
    // That's a bug. Either I add an operands Vec to the AST, or I
    // restructure Add as a flat sequence of (op, operand) pairs.
    //
    // See the note at the top of ast.rs for the final shape.
    // ========================================================================

    /// `OrExpr → AndExpr OrTail`
    ///
    /// Implementation: collect operators and operands in lockstep into
    /// parallel Vecs, then assemble the AST as `Expr::Or(Chain)`.
    fn parse_or_expr(&mut self) -> Expr {
        let left = self.parse_and_expr();
        let mut ops: Vec<LogicalOpChain> = Vec::new();
        let mut operands: Vec<Expr> = Vec::new();

        while self.stream.consume(TokenKind::OrOr).is_some() {
            ops.push(LogicalOpChain::Or);
            operands.push(self.parse_and_expr());
        }

        Expr::Or(Chain { left: Box::new(left), ops, operands })
    }

    /// `AndExpr → NotExpr AndTail`
    fn parse_and_expr(&mut self) -> Expr {
        let left = self.parse_not_expr();
        let mut ops: Vec<LogicalOpChain> = Vec::new();
        let mut operands: Vec<Expr> = Vec::new();

        while self.stream.consume(TokenKind::AndAnd).is_some() {
            ops.push(LogicalOpChain::And);
            operands.push(self.parse_not_expr());
        }

        Expr::And(Chain { left: Box::new(left), ops, operands })
    }

    /// `NotExpr → '!' NotExpr | CmpExpr`
    fn parse_not_expr(&mut self) -> Expr {
        if self.stream.consume(TokenKind::Bang).is_some() {
            let operand = self.parse_not_expr();
            Expr::Not {
                operand: Box::new(operand),
            }
        } else {
            self.parse_cmp_expr()
        }
    }

    /// `CmpExpr → AddExpr CmpTail`
    ///
    /// Comparison is non-associative, so at most one operator and one
    /// right operand. Stored as `Compare { left, op, right }`.
    fn parse_cmp_expr(&mut self) -> Expr {
        let left = self.parse_add_expr();
        let op = self.match_cmp_op();
        match op {
            None => left,
            Some(op) => {
                let right = self.parse_add_expr();
                Expr::Compare {
                    left: Box::new(left),
                    op: Some(op),
                    right: Some(Box::new(right)),
                }
            }
        }
    }

    fn match_cmp_op(&mut self) -> Option<CmpOp> {
        if self.stream.consume(TokenKind::EqualEqual).is_some() {
            Some(CmpOp::Eq)
        } else if self.stream.consume(TokenKind::NotEqual).is_some() {
            Some(CmpOp::Ne)
        } else if self.stream.consume(TokenKind::LessEqual).is_some() {
            Some(CmpOp::Le)
        } else if self.stream.consume(TokenKind::GreaterEqual).is_some() {
            Some(CmpOp::Ge)
        } else if self.stream.consume(TokenKind::Less).is_some() {
            Some(CmpOp::Lt)
        } else if self.stream.consume(TokenKind::Greater).is_some() {
            Some(CmpOp::Gt)
        } else {
            None
        }
    }

    /// `AddExpr → MulExpr AddTail`
    fn parse_add_expr(&mut self) -> Expr {
        let left = self.parse_mul_expr();
        let mut ops: Vec<AddOpChain> = Vec::new();
        let mut operands: Vec<Expr> = Vec::new();

        loop {
            if self.stream.consume(TokenKind::Plus).is_some() {
                ops.push(AddOpChain::Plus);
                operands.push(self.parse_mul_expr());
            } else if self.stream.consume(TokenKind::Minus).is_some() {
                ops.push(AddOpChain::Minus);
                operands.push(self.parse_mul_expr());
            } else {
                break;
            }
        }

        Expr::Add(Chain { left: Box::new(left), ops, operands })
    }

    /// `MulExpr → Unary MulTail`
    fn parse_mul_expr(&mut self) -> Expr {
        let left = self.parse_unary();
        let mut ops: Vec<MulOpChain> = Vec::new();
        let mut operands: Vec<Expr> = Vec::new();

        loop {
            if self.stream.consume(TokenKind::Star).is_some() {
                ops.push(MulOpChain::Times);
                operands.push(self.parse_unary());
            } else if self.stream.consume(TokenKind::Slash).is_some() {
                ops.push(MulOpChain::Div);
                operands.push(self.parse_unary());
            } else if self.stream.consume(TokenKind::Percent).is_some() {
                ops.push(MulOpChain::Mod);
                operands.push(self.parse_unary());
            } else {
                break;
            }
        }

        Expr::Mul(Chain { left: Box::new(left), ops, operands })
    }

    /// `Unary → '-' Unary | Primary`
    fn parse_unary(&mut self) -> Expr {
        if self.stream.consume(TokenKind::Minus).is_some() {
            let operand = self.parse_unary();
            Expr::Negate {
                operand: Box::new(operand),
            }
        } else {
            self.parse_primary()
        }
    }

    /// `Primary → INT | STRING | 'true' | 'false' | ID | '(' Expr ')'`
    fn parse_primary(&mut self) -> Expr {
        let token = match self.stream.advance() {
            Some(t) => t,
            None => {
                self.record_error("Expected expression, found end of input");
                return Expr::Primary(Literal::Int("0".into()));
            }
        };
        match token.kind {
            TokenKind::Integer => Expr::Primary(Literal::Int(token.lexeme)),
            TokenKind::StringLiteral => Expr::Primary(Literal::Str(token.lexeme)),
            TokenKind::KeywordTrue => Expr::Primary(Literal::True),
            TokenKind::KeywordFalse => Expr::Primary(Literal::False),
            TokenKind::Identifier => Expr::Primary(Literal::Ident(token.lexeme)),
            TokenKind::LeftParen => {
                let inner = self.parse_expr();
                if self.stream.consume(TokenKind::RightParen).is_none() {
                    self.record_error("Expected ')' to close grouped expression");
                }
                Expr::Primary(Literal::Grouped(inner))
            }
            _ => {
                self.record_error_at(
                    &token,
                    format!("Expected expression, found '{}'", token.lexeme),
                );
                Expr::Primary(Literal::Int("0".into()))
            }
        }
    }
}

// ============================================================================
// Pretty-printer (at the bottom so the parser code stays focused on parsing).
// ============================================================================

fn indent(n: usize) -> String {
    "  ".repeat(n)
}

fn render_stmt(out: &mut String, stmt: &Stmt, depth: usize) {
    match stmt {
        Stmt::FnDecl { body } => {
            out.push_str(&format!("{}Stmt[FnDecl]\n", indent(depth)));
            out.push_str(&format!("{}  'fn' 'neko'\n", indent(depth)));
            out.push_str(&format!("{}  Block ({} stmts):\n", indent(depth), body.len()));
            for inner in body {
                render_stmt(out, inner, depth + 2);
            }
        }
        Stmt::VarDecl { name, value } => {
            out.push_str(&format!("{}Stmt[VarDecl name={}]\n", indent(depth), name));
            render_expr(out, value, depth + 1);
        }
        Stmt::PrintStmt { value } => {
            out.push_str(&format!("{}Stmt[PrintStmt meow()]\n", indent(depth)));
            render_expr(out, value, depth + 1);
        }
    }
}

fn render_expr(out: &mut String, expr: &Expr, depth: usize) {
    match expr {
        Expr::If { condition, then_branch, else_branch } => {
            out.push_str(&format!("{}Expr[If]\n", indent(depth)));
            render_expr(out, condition, depth + 1);
            out.push_str(&format!("{}  then:\n", indent(depth)));
            render_expr(out, then_branch, depth + 2);
            if let Some(e) = else_branch {
                out.push_str(&format!("{}  else:\n", indent(depth)));
                render_expr(out, e, depth + 2);
            }
        }
        Expr::Match { scrutinee, arms } => {
            out.push_str(&format!("{}Expr[Match]\n", indent(depth)));
            render_expr(out, scrutinee, depth + 1);
            for (i, arm) in arms.iter().enumerate() {
                out.push_str(&format!(
                    "{}  arm[{}] pattern={:?} =>\n",
                    indent(depth),
                    i,
                    arm.pattern
                ));
                render_expr(out, &arm.body, depth + 2);
            }
        }
        Expr::Or(chain) => {
            out.push_str(&format!(
                "{}Expr[Or chain ops={}]\n",
                indent(depth),
                chain.ops.len()
            ));
            render_expr(out, &chain.left, depth + 1);
            for (i, operand) in chain.operands.iter().enumerate() {
                let op_str = match chain.ops.get(i) {
                    Some(LogicalOpChain::Or) => "||",
                    Some(LogicalOpChain::And) => "&&",
                    None => "?",
                };
                out.push_str(&format!("{}  {}\n", indent(depth), op_str));
                render_expr(out, operand, depth + 1);
            }
        }
        Expr::And(chain) => {
            out.push_str(&format!(
                "{}Expr[And chain ops={}]\n",
                indent(depth),
                chain.ops.len()
            ));
            render_expr(out, &chain.left, depth + 1);
            for (i, operand) in chain.operands.iter().enumerate() {
                let op_str = match chain.ops.get(i) {
                    Some(LogicalOpChain::And) => "&&",
                    Some(LogicalOpChain::Or) => "||",
                    None => "?",
                };
                out.push_str(&format!("{}  {}\n", indent(depth), op_str));
                render_expr(out, operand, depth + 1);
            }
        }
        Expr::Not { operand } => {
            out.push_str(&format!("{}Expr[Not]\n", indent(depth)));
            render_expr(out, operand, depth + 1);
        }
        Expr::Compare { left, op, right } => {
            let label = match op {
                Some(CmpOp::Eq) => "Eq",
                Some(CmpOp::Ne) => "Ne",
                Some(CmpOp::Lt) => "Lt",
                Some(CmpOp::Gt) => "Gt",
                Some(CmpOp::Le) => "Le",
                Some(CmpOp::Ge) => "Ge",
                None => "Compare (no op)",
            };
            out.push_str(&format!("{}Expr[{}]\n", indent(depth), label));
            render_expr(out, left, depth + 1);
            if let Some(r) = right {
                render_expr(out, r, depth + 1);
            }
        }
        Expr::Add(chain) => {
            out.push_str(&format!(
                "{}Expr[Add chain ops={}]\n",
                indent(depth),
                chain.ops.len()
            ));
            render_expr(out, &chain.left, depth + 1);
            for (i, operand) in chain.operands.iter().enumerate() {
                let op_str = match chain.ops.get(i) {
                    Some(AddOpChain::Plus) => "+",
                    Some(AddOpChain::Minus) => "-",
                    None => "?",
                };
                out.push_str(&format!("{}  {}\n", indent(depth), op_str));
                render_expr(out, operand, depth + 1);
            }
        }
        Expr::Mul(chain) => {
            out.push_str(&format!(
                "{}Expr[Mul chain ops={}]\n",
                indent(depth),
                chain.ops.len()
            ));
            render_expr(out, &chain.left, depth + 1);
            for (i, operand) in chain.operands.iter().enumerate() {
                let op_str = match chain.ops.get(i) {
                    Some(MulOpChain::Times) => "*",
                    Some(MulOpChain::Div) => "/",
                    Some(MulOpChain::Mod) => "%",
                    None => "?",
                };
                out.push_str(&format!("{}  {}\n", indent(depth), op_str));
                render_expr(out, operand, depth + 1);
            }
        }
        Expr::Negate { operand } => {
            out.push_str(&format!("{}Expr[Negate -]\n", indent(depth)));
            render_expr(out, operand, depth + 1);
        }
        Expr::Primary(lit) => {
            let s = match lit {
                Literal::Int(s) => format!("Int({})", s),
                Literal::Str(s) => format!("Str({})", s),
                Literal::True => "True".into(),
                Literal::False => "False".into(),
                Literal::Ident(s) => format!("Ident({})", s),
                Literal::Grouped(inner) => {
                    out.push_str(&format!("{}Primary[Grouped]\n", indent(depth)));
                    render_expr(out, inner, depth + 1);
                    return;
                }
            };
            out.push_str(&format!("{}Primary[{}]\n", indent(depth), s));
        }
    }
}
