// ============================================================================
// Quadruple — one of the two intermediate-code representations produced by
// the ICG. Classic three-address-code tuple of 4 fields:
//
//     ( operator , arg1 , arg2 , result )
//
// where:
//   - operator is a Neko op or a pseudo-op (GOTO, GOTOF, GOTOV, PRINT, LABEL)
//   - arg1, arg2 are operands: either user variables (e.g. `a`, `x`),
//     literals (e.g. `90`, `"Hola"`), or compiler temporaries (`T1`, `T2`).
//     Unused slots are represented by the string "_".
//   - result is the destination: a temporary (`T_n`) for arithmetic ops,
//     a label (`L_n`) for jumps, or "_" when the op has no result
//     (e.g. GOTO, PRINT).
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quadruple {
    pub op: String,
    pub arg1: String,
    pub arg2: String,
    pub result: String,
}

impl Quadruple {
    pub fn new(op: impl Into<String>, arg1: impl Into<String>, arg2: impl Into<String>, result: impl Into<String>) -> Self {
        Self {
            op: op.into(),
            arg1: arg1.into(),
            arg2: arg2.into(),
            result: result.into(),
        }
    }

    /// Pretty-print with columns aligned to width 6.
    /// Example output: ` ( + ,    a,    b,   T1 )`
    pub fn render(&self) -> String {
        format!(
            " ( {:>6}, {:>6}, {:>6}, {:>6} )",
            self.op, self.arg1, self.arg2, self.result
        )
    }
}
