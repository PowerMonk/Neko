// ============================================================================
// Codegen Symbol Table
//
// The symbol table for the code generation phase. It answers the question
// "WHERE does this name live in memory?" — the counterpart to the
// quadruple table, which answers "WHAT does the program do?".
//
// The teacher's notes for this phase say:
//
//   "las direcciones de los datos se obtendrían de la tabla de símbolos"
//   "the symbols table must obviously contain the users input like the
//    keyboard"
//
// So this table:
//   1. Assigns an address (section + offset) to every name the program
//      uses: user variables, compiler temporaries, and string literals.
//   2. Is PRE-LOADED with the runtime symbols — including the keyboard
//      input buffer — so the generated program always has them available.
//
// Memory model
// ------------
// Two sections, exactly like a classic assembly program:
//
//   section .data  → initialised data: string literals, the "true"/"false"
//                    text, and the runtime's static tables.
//   section .bss   → uninitialised data: user variables, compiler
//                    temporaries, the keyboard input buffer, and the
//                    scratch buffer used to convert integers to text.
//
// Both sections are laid out sequentially. A symbol's "address" is its
// byte offset within its section — the classic relocatable-address model
// used by every assembler and linker. The assembler emits the LABEL
// name; the assembler/linker resolves it to a real address.
//
// Stack
// -----
// The stack is NOT part of this table. It is reserved once at program
// start with a fixed size (see `STACK_SIZE` in the generated file). In
// this iteration of Neko there is exactly one function (`fn neko`) with
// no parameters, so there are no frames to manage — the whole program
// runs in one flat stack frame.
// ============================================================================

use crate::semantic::Type;

/// Which section of the generated program a symbol lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// `.data` — initialised at load time.
    Data,
    /// `.bss` — reserved at load time, zero-filled by the OS.
    Bss,
}

impl Section {
    pub fn name(self) -> &'static str {
        match self {
            Section::Data => ".data",
            Section::Bss => ".bss",
        }
    }
}

/// What a symbol represents. Determines how it is emitted in the .asm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// A user variable declared with `nyan`.
    Variable,
    /// A compiler temporary (`T1`, `T2`, ...).
    Temporary,
    /// A string literal in the program.
    StringLiteral,
    /// A pre-loaded runtime symbol (keyboard buffer, scratch buffers...).
    Runtime,
}

/// The unit in which a symbol's size is expressed.
///
/// This matters for the assembler directives: `resb 256` reserves 256
/// BYTES, while `resq 256` would reserve 256 quadwords (2048 bytes).
/// Getting this wrong silently allocates the wrong amount of memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// A byte buffer (`resb`).
    Bytes,
    /// A quadword slot — every Neko value fits in 8 bytes (`resq`).
    Quadwords,
}

/// One entry in the symbol table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub section: Section,
    /// Byte offset within the section. This is the symbol's "address".
    pub offset: usize,
    /// Size in bytes.
    pub size: usize,
    /// The unit used to express the size in the assembler directive.
    pub unit: Unit,
    pub kind: SymbolKind,
    /// The static type, when known. Used to decide how to print a value.
    pub ty: Option<Type>,
}

impl Symbol {
    /// The label the assembler will use for this symbol.
    pub fn label(&self) -> String {
        // Labels must be valid NASM identifiers. User variable names in
        // Neko are already valid identifiers, but to avoid any collision
        // with the runtime's own labels (which start with `neko_`), we
        // prefix user symbols.
        match self.kind {
            SymbolKind::Runtime => self.name.clone(),
            SymbolKind::Temporary => self.name.clone(),
            _ => format!("var_{}", self.name),
        }
    }
}

/// The symbol table for the code generation phase.
#[derive(Debug, Clone)]
pub struct SymbolTable {
    symbols: Vec<Symbol>,
    /// Current write offset in `.data`.
    data_offset: usize,
    /// Current write offset in `.bss`.
    bss_offset: usize,
    /// Literal text of every string in `.data`, in emission order.
    /// The assembler uses this to write the actual `db` directives.
    data_strings: Vec<(String, String)>,
}

impl SymbolTable {
    /// Create a symbol table pre-loaded with the runtime symbols.
    ///
    /// The runtime symbols are the ones the generated program always
    /// needs. The most important one is `stdin`: the keyboard input
    /// buffer, which the teacher's notes explicitly call for.
    pub fn with_runtime() -> Self {
        let mut table = SymbolTable {
            symbols: Vec::new(),
            data_offset: 0,
            bss_offset: 0,
            data_strings: Vec::new(),
        };

        // ---- .bss: the keyboard input buffer and friends ----
        // `stdin` is the buffer where `neko_read_line` deposits the text
        // typed by the user. It is 256 bytes, plenty for a console.
        table.declare_runtime("stdin", 256, Unit::Bytes);
        // `stdin_len` holds how many bytes the last read returned.
        table.declare_runtime("stdin_len", 8, Unit::Quadwords);
        // `stdout` holds the file descriptor used for output (1).
        table.declare_runtime("stdout", 8, Unit::Quadwords);
        // `numbuf` is the scratch buffer used by `neko_print_int` to build
        // the decimal representation of an integer.
        table.declare_runtime("numbuf", 256, Unit::Bytes);

        table
    }

    /// Declare a runtime symbol in `.bss`. Runtime symbols keep their own
    /// name (no `var_` prefix) because the generated runtime routines
    /// refer to them directly.
    fn declare_runtime(&mut self, name: &str, size: usize, unit: Unit) {
        let symbol = Symbol {
            name: name.to_string(),
            section: Section::Bss,
            offset: self.bss_offset,
            size,
            unit,
            kind: SymbolKind::Runtime,
            ty: None,
        };
        self.bss_offset += size;
        self.symbols.push(symbol);
    }

    /// Declare a user variable in `.bss`.
    ///
    /// Variables live in `.bss` because their value is assigned at
    /// RUNTIME by the `=` quadruple — not baked into the binary. The
    /// initial value of zero is what the OS gives us for `.bss`.
    pub fn declare_variable(&mut self, name: &str, ty: Type) {
        let symbol = Symbol {
            name: name.to_string(),
            section: Section::Bss,
            offset: self.bss_offset,
            size: 8, // every Neko value fits in a qword (8 bytes)
            unit: Unit::Quadwords,
            kind: SymbolKind::Variable,
            ty: Some(ty),
        };
        self.bss_offset += 8;
        self.symbols.push(symbol);
    }

    /// Declare a compiler temporary in `.bss`.
    pub fn declare_temporary(&mut self, name: &str) {
        let symbol = Symbol {
            name: name.to_string(),
            section: Section::Bss,
            offset: self.bss_offset,
            size: 8,
            unit: Unit::Quadwords,
            kind: SymbolKind::Temporary,
            ty: None,
        };
        self.bss_offset += 8;
        self.symbols.push(symbol);
    }

    /// Declare a string literal in `.data`.
    ///
    /// Returns the label the assembler will use. Identical literals are
    /// stored only once (the same text always gets the same label).
    pub fn declare_string(&mut self, literal: &str) -> String {
        // Look for an existing literal with the same text.
        for (label, text) in &self.data_strings {
            if text == literal {
                return label.clone();
            }
        }

        // Not seen yet — allocate it.
        let label = format!("str_{}", self.data_strings.len());
        let size = literal.len() + 1; // +1 for the NUL terminator
        let symbol = Symbol {
            name: label.clone(),
            section: Section::Data,
            offset: self.data_offset,
            size,
            unit: Unit::Bytes,
            kind: SymbolKind::StringLiteral,
            ty: Some(Type::String),
        };
        self.data_offset += size;
        self.symbols.push(symbol);
        self.data_strings.push((label.clone(), literal.to_string()));
        label
    }

    /// Look up a symbol by name.
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.name == name)
    }

    /// The section a symbol lives in, if it is known.
    #[allow(dead_code)]
    pub fn section_of(&self, name: &str) -> Option<Section> {
        self.lookup(name).map(|s| s.section)
    }

    /// Every symbol in the table, in declaration order.
    #[allow(dead_code)]
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// The string literals in `.data`, in emission order: (label, text).
    pub fn data_strings(&self) -> &[(String, String)] {
        &self.data_strings
    }

    /// Find the label of an already-declared string literal.
    ///
    /// Used by the assembler when it needs the address of a literal it
    /// has already seen. Returns `None` if the literal was never
    /// declared.
    pub fn lookup_string_literal(&self, text: &str) -> Option<String> {
        self.data_strings
            .iter()
            .find(|(_, t)| t == text)
            .map(|(label, _)| label.clone())
    }

    /// The symbols that live in `.bss`, in emission order.
    pub fn bss_symbols(&self) -> Vec<&Symbol> {
        self.symbols
            .iter()
            .filter(|s| s.section == Section::Bss)
            .collect()
    }

    /// The compiler temporaries, sorted by their numeric suffix.
    ///
    /// The discovery pass finds temporaries in the order they appear in
    /// the quadruple table, which is not necessarily `T1, T2, T3...`.
    /// Sorting them keeps the generated `.bss` readable and matches the
    /// order a reader expects.
    pub fn sorted_temporaries(&self) -> Vec<&Symbol> {
        let mut temporaries: Vec<&Symbol> = self
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Temporary)
            .collect();
        temporaries.sort_by_key(|s| {
            s.name
                .chars()
                .skip(1)
                .collect::<String>()
                .parse::<usize>()
                .unwrap_or(0)
        });
        temporaries
    }

    /// Total size of the `.bss` section.
    #[allow(dead_code)]
    pub fn bss_size(&self) -> usize {
        self.bss_offset
    }

    /// Render the symbol table as a listing for `output/symbol_table.txt`.
    ///
    /// Columns: name, kind, section, offset, size, type.
    ///
    /// The listing follows the same order as the generated sections: the
    /// runtime symbols first, then the user variables, then the compiler
    /// temporaries (sorted numerically, so they read `T1, T2, T3...`),
    /// then the string literals.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "{:<20} {:<14} {:<6} {:>6} {:>6}  {}\n",
            "NAME", "KIND", "SECTION", "OFFSET", "SIZE", "TYPE"
        ));
        out.push_str(&format!("{}\n", "-".repeat(72)));

        let mut ordered: Vec<&Symbol> = Vec::new();
        // Everything except the temporaries, in declaration order.
        ordered.extend(
            self.symbols
                .iter()
                .filter(|s| s.kind != SymbolKind::Temporary),
        );
        // The temporaries, sorted by their numeric suffix.
        ordered.extend(self.sorted_temporaries());

        for symbol in ordered {
            let kind = match symbol.kind {
                SymbolKind::Variable => "variable",
                SymbolKind::Temporary => "temporary",
                SymbolKind::StringLiteral => "string",
                SymbolKind::Runtime => "runtime",
            };
            let ty = match symbol.ty {
                Some(t) => t.name().to_string(),
                None => "-".to_string(),
            };
            out.push_str(&format!(
                "{:<20} {:<14} {:<6} {:>6} {:>6}  {}\n",
                symbol.name,
                kind,
                symbol.section.name(),
                symbol.offset,
                symbol.size,
                ty
            ));
        }
        out
    }
}
