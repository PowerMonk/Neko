// ============================================================================
// Neko Semantic Types
//
// The Neko type system, as committed to in `docs/06-semantic-conventions.md`.
// In this first iteration we have three basic types (`int`, `string`, `bool`)
// and one constructed type (`fn`, representing the `fn neko { … }` function).
// ============================================================================

/// Every value in a Neko program has a static type, represented by this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)] // `Fn` is reserved for a future iteration
pub enum Type {
    Int,
    String,
    Bool,
    /// The type of a `fn neko { … }` function. Always the same shape
    /// (no parameters, returns the type of the last statement of its
    /// body — but since every program has exactly one function, the
    /// return type is rarely inspected externally in this iteration).
    Fn,
}

impl Type {
    /// String form used in error messages: `int`, `string`, `bool`, `fn`.
    pub fn name(self) -> &'static str {
        match self {
            Type::Int => "int",
            Type::String => "string",
            Type::Bool => "bool",
            Type::Fn => "fn",
        }
    }
}
