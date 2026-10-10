// ============================================================================
// Triple — the second intermediate-code representation.
//
// A triple is a 3-field tuple:
//
//     ( operator , arg1 , arg2 )
//
// with NO explicit result field. The result of a triple is implicitly the
// position (index) where the triple appears in the list — so a later triple
// that uses the result just writes the index as a reference.
//
// Example — for `a + b * c`:
//   index 0:  (*, b, c)
//   index 1:  (+, a, (0))    <-- operand references triple 0
//
// We use string references like "(0)", "(1)" inside the arg fields when a
// later triple wants to consume the result of an earlier one. This is the
// textbook representation (Aho/Sethi/Ullman, "Dragon Book", §6.4.2).
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Triple {
    pub op: String,
    pub arg1: String,
    pub arg2: String,
}

impl Triple {
    pub fn new(op: impl Into<String>, arg1: impl Into<String>, arg2: impl Into<String>) -> Self {
        Self {
            op: op.into(),
            arg1: arg1.into(),
            arg2: arg2.into(),
        }
    }

    /// Pretty-print with the triple's index. The format matches the textbook:
    ///   (0) (*, b, c)
    ///   (1) (+, a, (0))
    pub fn render(&self, index: usize) -> String {
        format!(
            " ({:>3})  ({:>6}, {:>6}, {:>6} )",
            index, self.op, self.arg1, self.arg2
        )
    }
}
