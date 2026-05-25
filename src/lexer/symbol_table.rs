/// A compact symbol table that keeps the first appearance order of identifiers.
///
/// The project is educational, so a simple vector-backed table keeps the code
/// easy to read while still preventing duplicate symbol entries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolTable {
    identifiers: Vec<String>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            identifiers: Vec::new(),
        }
    }

    pub fn insert(&mut self, identifier: &str) {
        if !self.identifiers.iter().any(|entry| entry == identifier) {
            self.identifiers.push(identifier.to_owned());
        }
    }

    pub fn entries(&self) -> &[String] {
        &self.identifiers
    }

    pub fn render(&self) -> String {
        if self.identifiers.is_empty() {
            String::new()
        } else {
            let mut output = self.identifiers.join("\n");
            output.push('\n');
            output
        }
    }
}
