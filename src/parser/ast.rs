// ============================================================================
// Neko AST (Abstract Syntax Tree)
//
// Why this exists
// ---------------
// The grammar in docs/02-grammar.md is a *parse tree* spec: every production
// becomes a node in the tree, so even a simple expression like `score` ends
// up as Expr → OrExpr → AndExpr → NotExpr → CmpExpr → AddExpr → MulExpr
// → Unary → Primary → ID "score". That's the honest form for a parse tree,
// but it's enormous for what is semantically just "read the variable named
// score".
//
// For the AST we collapse every left-recursive *Tail chain
//   AddExpr → MulExpr AddTail
//   AddTail → '+' MulExpr AddTail | '-' MulExpr AddTail | ε
// into a single node carrying the LEADING operand (`left`) plus parallel
// Vecs of operators and operands:
//
//     Expr::Add {
//         left: <first operand>,
//         ops: [Plus, Minus, Plus, ...],
//         operands: [<2nd operand>, <3rd operand>, <4th operand>, ...],
//     }
//
// The pairing is positional: `ops[i]` sits between `left` (when i==0)
// or `operands[i-1]` (when i>0) and `operands[i]`.
//
// The tree in `docs/04-parse-tree-example.mmd` still shows the full
// descent — that's the *parse* tree. The AST is what semantic analysis
// and code generation will actually consume.
// ============================================================================

/// Top-level program: a sequence of statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

/// A statement — the three forms the grammar allows at the top level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    /// `fn neko { <stmts> }`  →  FnDecl → 'fn' 'neko' Block
    ///
    /// Note: the body is a sequence of statements (not a single Expr),
    /// because the grammar's `Block → '{' Stmts '}'` allows multiple
    /// declarations inside the function body. See docs/02-grammar.md.
    FnDecl { body: Vec<Stmt> },

    /// `nyan <id> = <expr>`  →  VarDecl → 'nyan' ID '=' Expr
    VarDecl { name: String, value: Box<Expr> },

    /// `meow(<expr>)`  →  PrintStmt → 'meow' '(' Expr ')'
    PrintStmt { value: Box<Expr> },
}

/// An expression — every value-producing construct in Neko.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// `if <expr> <block> else <block>`  (the else branch is optional)
    /// Grammar: `Expr → 'if' Expr Block ElsePart`
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },

    /// `match <expr> { <arms> }`
    /// Grammar: `Expr → 'match' Expr '{' Arms '}'`
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<Arm>,
    },

    /// A logical-OR chain.
    /// Grammar: `OrExpr → AndExpr OrTail`, flattened.
    Or(Chain<LogicalOpChain>),

    /// A logical-AND chain.
    And(Chain<LogicalOpChain>),

    /// A logical-NOT expression (may chain: `!!x`).
    /// Grammar: `NotExpr → '!' NotExpr | CmpExpr`
    Not { operand: Box<Expr> },

    /// A comparison expression. The `op` is `None` when no comparison
    /// operator was used (i.e. just `AddExpr`).
    /// Grammar: `CmpExpr → AddExpr CmpTail`, with CmpTail flattened.
    Compare {
        left: Box<Expr>,
        op: Option<CmpOp>,
        right: Option<Box<Expr>>,
    },

    /// An addition/subtraction chain. Flattened.
    Add(Chain<AddOpChain>),

    /// A multiplication/division/modulo chain. Flattened.
    Mul(Chain<MulOpChain>),

    /// A unary minus expression. Grammar: `Unary → '-' Unary | Primary`
    Negate { operand: Box<Expr> },

    /// A primary expression — a leaf in the AST.
    Primary(Literal),
}

/// A flattened left-associative chain at a single precedence level.
///
/// `left` is the first operand; `ops[i]` and `operands[i]` together
/// represent the (i+1)-th operand. For a single-operand "chain" (no
/// operators at this level), `ops` and `operands` are empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain<Op> {
    pub left: Box<Expr>,
    pub ops: Vec<Op>,
    pub operands: Vec<Expr>,
}

impl<Op> Chain<Op> {
    /// Build a chain from a single operand (no operators).
    /// The caller can wrap this in an Expr variant.
    #[allow(dead_code)]
    pub fn singleton(operand: Expr) -> Self {
        Chain {
            left: Box::new(operand),
            ops: Vec::new(),
            operands: Vec::new(),
        }
    }

    /// Number of operands in the chain (always >= 1).
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        1 + self.operands.len()
    }
}

/// Literal / primary forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Int(String),
    Str(String),
    True,
    False,
    Ident(String),
    /// Parenthesized expression — preserved as-is so codegen can re-emit
    /// the parens or just unwrap them.
    Grouped(Box<Expr>),
}

// ---------------------------------------------------------------------------
// Operator enums
//
// One enum per precedence level (instead of one big `BinOp`) so the AST
// enforces what combinations are even possible. `a + b == c` is an
// `Expr::Compare { left: Add(...), op: Eq, right: Ident(c) }` — the `==`
// step can only live in Compare, not Add.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOpChain {
    /// `'&&'` step in an AndExpr chain.
    And,
    /// `'||'` step in an OrExpr chain.
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,  // ==
    Ne,  // !=
    Lt,  // <
    Gt,  // >
    Le,  // <=
    Ge,  // >=
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddOpChain {
    Plus,
    Minus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MulOpChain {
    Times,
    Div,
    Mod,
}

// ---------------------------------------------------------------------------
// Patterns (used in `match` arms) and the arm itself.
// ---------------------------------------------------------------------------

/// A pattern on the left side of a `match` arm.
///
/// Grammar:
/// ```
/// Pattern → '_' | INT | STRING | ID | 'true' | 'false'
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Wildcard,
    Int(String),
    Str(String),
    /// Binding pattern — the matched value is bound to this identifier name.
    Bind(String),
    True,
    False,
}

/// One arm of a `match` expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arm {
    pub pattern: Pattern,
    pub body: Box<Expr>,
}
