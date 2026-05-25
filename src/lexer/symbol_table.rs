// A compact symbol table that keeps identifiers in order of first appearance.
//
// In a production compiler this would use a HashMap for O(1) lookups, but for
// an educational compiler a linear scan through a Vec is clearer to read.

// `#[derive(Default)]` lets us create an empty SymbolTable with `Default::default()`.
// Since `Vec<T>` implements `Default` (returns an empty Vec), the derive works.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolTable {
    // Private field — external code cannot read this directly.
    // The `pub` methods (`insert`, `entries`, `render`) control access.
    identifiers: Vec<String>,  // `Vec<String>` = a growable array of owned strings
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            identifiers: Vec::new(),
        }
    }

    // `&mut self` — mutable borrow: this method can modify the struct.
    // `&str` — a borrowed string slice (like a read-only reference to part/all of a String).
    //          In TS, strings are always references; in Rust, you distinguish
    //          owned (`String`) from borrowed (`&str`).
    pub fn insert(&mut self, identifier: &str) {
        // `.iter()` creates an iterator over references to the Vec elements.
        // `.any(|entry| entry == identifier)` checks if any entry equals the given string.
        // Rust allows comparing `&String` with `&str` via the `PartialEq` trait.
        if !self.identifiers.iter().any(|entry| entry == identifier) {
            // `.to_owned()` creates an owned `String` copy from the `&str` borrow.
            // This is necessary because `Vec<String>` stores owned Strings, not borrows.
            self.identifiers.push(identifier.to_owned());
        }
    }

    // Returns `&[String]` — a slice reference (like borrowing a read-only array).
    // The caller can iterate over the entries but cannot modify them.
    pub fn entries(&self) -> &[String] {
        &self.identifiers  // `&Vec<String>` auto-converts to `&[String]` via Deref
    }

    // Renders all identifiers joined by newlines, for writing to the output file.
    pub fn render(&self) -> String {
        if self.identifiers.is_empty() {
            String::new()
        } else {
            // `.join("\n")` is like `arr.join("\n")` in TS.
            let mut output = self.identifiers.join("\n");
            output.push('\n');
            output
        }
    }
}
