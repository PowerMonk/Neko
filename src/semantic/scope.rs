// ============================================================================
// Scope & Symbol Table
//
// Neko's scope rules live here. Two things matter:
//
//   1. Names are visible from their declaration to the end of the
//      enclosing block (block scoping).
//
//   2. Neko has NO SHADOWING. A name declared in an inner scope cannot
//      have the same name as a binding in any outer scope. See
//      `docs/06-semantic-conventions.md` §3.
//
// Implementation: a stack of `HashMap<String, Symbol>` with parent
// links. `lookup` walks the chain upward; `declare` checks the entire
// chain (not just the current scope) for an existing binding so that
// shadowing is caught at declaration time, not at use time.
// ============================================================================

use super::types::Type;

/// One declared name and the type it was declared with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub ty: Type,
}

impl Symbol {
    pub fn new(name: impl Into<String>, ty: Type) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

/// One scope level. Owns its bindings; has an optional pointer to the
/// enclosing scope so `lookup` can walk upward.
#[derive(Debug, Clone)]
pub struct Scope {
    bindings: std::collections::HashMap<String, Symbol>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    /// Create a fresh top-level scope with no parent.
    pub fn new_root() -> Self {
        Self {
            bindings: std::collections::HashMap::new(),
            parent: None,
        }
    }

    /// Create a child scope whose parent is `self`.
    pub fn child(&self) -> Self {
        Self {
            bindings: std::collections::HashMap::new(),
            parent: Some(Box::new(self.clone())),
        }
    }

    /// Insert a binding in THIS scope only.
    ///
    /// Returns `Err(())` if a binding with the same name already exists
    /// in this exact scope. (Shadowing across scopes is checked
    /// separately by `would_shadow`.)
    pub fn insert_local(&mut self, name: &str, symbol: Symbol) -> Result<(), ()> {
        if self.bindings.contains_key(name) {
            return Err(());
        }
        self.bindings.insert(name.to_string(), symbol);
        Ok(())
    }

    /// Walks the parent chain and returns `true` if `name` exists in
    /// ANY enclosing scope (including the current one).
    pub fn contains_in_chain(&self, name: &str) -> bool {
        if self.bindings.contains_key(name) {
            return true;
        }
        match &self.parent {
            Some(p) => p.contains_in_chain(name),
            None => false,
        }
    }

    /// Walks the parent chain and returns the first matching `Symbol`.
    pub fn lookup(&self, name: &str) -> Option<Symbol> {
        if let Some(sym) = self.bindings.get(name) {
            return Some(sym.clone());
        }
        match &self.parent {
            Some(p) => p.lookup(name),
            None => None,
        }
    }

    /// Returns `true` if `name` is bound in THIS scope only (not in any
    /// parent). Useful to distinguish "redeclaration in same scope" from
    /// "shadowing an outer scope".
    pub fn contains_local(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Borrow the parent scope (immutable).
    pub fn parent_ref(&self) -> Option<&Scope> {
        self.parent.as_deref()
    }

    /// Take the parent scope out of this scope, leaving `self` in a
    /// dropped state. Used by the analyzer to implement `pop_scope()`
    /// without cloning the whole chain.
    #[allow(dead_code)]
    pub fn into_parent(self) -> Option<Scope> {
        self.parent.map(|p| *p)
    }
}
