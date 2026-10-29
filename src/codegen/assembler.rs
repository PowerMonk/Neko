// ============================================================================
// Assembler — translates the intermediate code (quadruples) into x86-64
// NASM assembly.
//
// Input
// -----
//   * `Vec<Quadruple>` — the 3-address code produced by the ICG
//     (`src/icg/`). Says WHAT the program does.
//   * The semantic symbol table (`name -> Type`). Says the static TYPE of
//     every user variable, which the quadruple table does not carry.
//
// Output
// ------
//   A complete, self-contained NASM file that can be assembled with
//   `nasm -f elf64` and linked with `ld` to produce a runnable program.
//
// How the two tables work together
// --------------------------------
// The quadruple table names things: "add `a` and `b` into `T1`". It does
// not say where `a`, `b` or `T1` live. That is the symbol table's job
// (see `symbol_table.rs`). This module is where the two meet: every
// name mentioned in a quadruple is looked up in the symbol table, and the
// result is a memory reference.
//
// Memory model
// ------------
//   section .data  → string literals, plus the "true"/"false" texts
//   section .bss   → user variables, compiler temporaries, and the
//                    runtime buffers (keyboard input, scratch space)
//   section .text  → the program, plus the runtime routines
//
// The stack is reserved once at program start with a fixed size. Neko has
// a single function (`fn neko`) with no parameters, so there are no
// frames to manage: the whole program runs in one flat stack frame.
//
// Instruction patterns
// --------------------
// Each quadruple maps to a small, fixed instruction sequence:
//
//   ( +, a, b, T )   mov rax, a / mov rcx, b / add rax, rcx / mov [T], rax
//   ( /, a, b, T )   mov rax, a / cqo / idiv rcx / mov [T], rax
//   ( %, a, b, T )   mov rax, a / cqo / idiv rcx / mov [T], rdx
//   ( ==, a, b, T )  mov rax, a / mov rcx, b / cmp rax, rcx
//                    / sete al / movzx rax, al / mov [T], rax
//   ( &&, a, b, T )  mov rax, a / mov rcx, b / and rax, rcx / mov [T], rax
//   ( !, a, _, T )   mov rax, a / xor rax, 1 / mov [T], rax
//   ( =, s, _, d )   mov rax, s / mov [d], rax
//   ( PRINT, x )     mov rdi, x / call neko_print_{str,int,bool}
//   ( GOTO, _, _, L )  jmp L
//   ( GOTOF, c, _, L ) mov rax, c / test rax, rax / jz L
//   ( GOTOV, c, _, L ) mov rax, c / test rax, rax / jnz L
//   ( LABEL, _, _, L )  L:
//
// Note on strings: a Neko string value is represented by the ADDRESS of
// its text. So `(=, "Hola", _, x)` stores the address of the literal into
// `x`, and `PRINT x` loads that address back and hands it to
// `neko_print_str`.
// ============================================================================

use super::runtime;
use super::symbol_table::{SymbolKind, SymbolTable, Unit};

use crate::icg::quadruple::Quadruple;
use crate::semantic::Type;

/// Size in bytes reserved for the stack at program start.
pub const STACK_SIZE: usize = 4096;

/// How an operand in a quadruple should be interpreted.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Operand {
    /// An integer literal, e.g. `42`.
    IntLiteral(i64),
    /// A string literal, e.g. `"Hola"` (the surrounding quotes are
    /// stripped; this holds the text only).
    StrLiteral(String),
    /// A boolean literal: `true` or `false`.
    BoolLiteral(bool),
    /// A user variable — its address comes from the symbol table.
    Variable(String),
    /// A compiler temporary (`T1`, `T2`, ...).
    Temporary(String),
    /// A jump label (`L1`, `L2`, ...).
    Label(String),
    /// The `_` placeholder (an unused slot).
    Unused,
}

/// The assembly generator.
pub struct Assembler {
    /// The symbol table being built.
    symbols: SymbolTable,
    /// The type of every compiler temporary, filled in as we translate.
    temp_types: std::collections::HashMap<String, Type>,
    /// The type of every user variable, taken from the semantic phase.
    var_types: std::collections::HashMap<String, Type>,
}

impl Assembler {
    /// Create an assembler.
    ///
    /// `var_types` is the semantic phase's symbol table (name -> Type).
    pub fn new(var_types: std::collections::HashMap<String, Type>) -> Self {
        Self {
            symbols: SymbolTable::with_runtime(),
            temp_types: std::collections::HashMap::new(),
            var_types,
        }
    }

    /// Translate the quadruples into a complete assembly file.
    pub fn assemble(&mut self, quadruples: &[Quadruple]) -> String {
        // ---- Pass 1: discover every name the program mentions, so we
        //      can lay out the symbol table before emitting code. ----
        self.declare_all_symbols(quadruples);

        // ---- Pass 2: translate each quadruple into instructions. ----
        let mut body = String::new();
        for quad in quadruples {
            self.translate_quadruple(quad, &mut body);
        }

        self.render(body)
    }

    // ========================================================================
    // Pass 1 — symbol discovery
    // ========================================================================

    /// Scan every quadruple and declare the symbols it needs.
    fn declare_all_symbols(&mut self, quadruples: &[Quadruple]) {
        // Variables first (in order of first appearance), then the
        // temporaries, then the string literals. The order determines the
        // layout of the `.bss` and `.data` sections.
        let mut variables: Vec<String> = Vec::new();
        let mut temporaries: Vec<String> = Vec::new();
        let mut strings: Vec<String> = Vec::new();

        for quad in quadruples {
            // The destination of an assignment is a variable or a temp.
            for field in [&quad.arg1, &quad.arg2, &quad.result] {
                match self.classify(field) {
                    Operand::Variable(name) => {
                        if !variables.contains(&name) {
                            variables.push(name);
                        }
                    }
                    Operand::Temporary(name) => {
                        if !temporaries.contains(&name) {
                            temporaries.push(name);
                        }
                    }
                    Operand::StrLiteral(text) => {
                        if !strings.contains(&text) {
                            strings.push(text);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Declare the user variables. Their static type comes from the
        // semantic phase.
        for name in &variables {
            let ty = self.var_types.get(name).copied().unwrap_or(Type::Int);
            self.symbols.declare_variable(name, ty);
        }

        // Declare the compiler temporaries.
        for name in &temporaries {
            self.symbols.declare_temporary(name);
        }

        // Declare the string literals (deduplicated by the symbol table).
        for text in &strings {
            self.symbols.declare_string(text);
        }
    }

    // ========================================================================
    // Operand classification
    // ========================================================================

    /// Decide what an operand field of a quadruple means.
    fn classify(&self, text: &str) -> Operand {
        if text == "_" {
            return Operand::Unused;
        }
        if text == "true" {
            return Operand::BoolLiteral(true);
        }
        if text == "false" {
            return Operand::BoolLiteral(false);
        }
        // A string literal keeps its surrounding quotes in the token's
        // lexeme, so we can spot it by them.
        if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
            return Operand::StrLiteral(text[1..text.len() - 1].to_string());
        }
        // A user variable takes precedence over the temporary pattern, so
        // a program with a variable called `T1` still works.
        if self.var_types.contains_key(text) {
            return Operand::Variable(text.to_string());
        }
        // Temporaries: `T` followed by digits.
        if is_temporary_name(text) {
            return Operand::Temporary(text.to_string());
        }
        // Labels: `L` followed by digits.
        if is_label_name(text) {
            return Operand::Label(text.to_string());
        }
        // A plain integer literal.
        if let Ok(number) = text.parse::<i64>() {
            return Operand::IntLiteral(number);
        }
        // Anything else is treated as a variable (the semantic phase
        // already rejected anything undeclared).
        Operand::Variable(text.to_string())
    }

    // ========================================================================
    // Pass 2 — translation
    // ========================================================================

    /// Translate one quadruple into assembly instructions.
    fn translate_quadruple(&mut self, quad: &Quadruple, out: &mut String) {
        let op = quad.op.as_str();

        match op {
            // ---- Unary minus ----
            // MUST come before the binary-arithmetic arm: it is
            // distinguished from binary minus by `arg2` being "_".
            // (Rust matches arms in order, so putting it later would make
            // it unreachable.)
            "-" if quad.arg2 == "_" => {
                self.emit_load(out, &quad.arg1, "rax");
                out.push_str("    neg rax\n");
                self.emit_store(out, &quad.result, "rax");
                self.record_temp_type(&quad.result, Type::Int);
            }

            // ---- Arithmetic ----
            "+" | "-" | "*" => {
                if op == "*" {
                    self.emit_load(out, &quad.arg1, "rax");
                    self.emit_load(out, &quad.arg2, "rcx");
                    out.push_str("    imul rax, rcx\n");
                } else {
                    self.emit_load(out, &quad.arg1, "rax");
                    self.emit_load(out, &quad.arg2, "rcx");
                    if op == "+" {
                        out.push_str("    add rax, rcx\n");
                    } else {
                        out.push_str("    sub rax, rcx\n");
                    }
                }
                self.emit_store(out, &quad.result, "rax");
                self.record_temp_type(&quad.result, Type::Int);
            }

            // Division and modulo use the x86 signed-division idiom:
            // sign-extend rax into rdx:rax with `cqo`, then `idiv rcx`.
            "/" => {
                self.emit_load(out, &quad.arg1, "rax");
                self.emit_load(out, &quad.arg2, "rcx");
                out.push_str("    cqo\n");
                out.push_str("    idiv rcx\n");
                self.emit_store(out, &quad.result, "rax");
                self.record_temp_type(&quad.result, Type::Int);
            }
            "%" => {
                self.emit_load(out, &quad.arg1, "rax");
                self.emit_load(out, &quad.arg2, "rcx");
                out.push_str("    cqo\n");
                out.push_str("    idiv rcx\n");
                // The remainder is left in rdx.
                self.emit_store(out, &quad.result, "rdx");
                self.record_temp_type(&quad.result, Type::Int);
            }

            // ---- Comparison ----
            "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                self.emit_load(out, &quad.arg1, "rax");
                self.emit_load(out, &quad.arg2, "rcx");
                out.push_str("    cmp rax, rcx\n");
                let cc = match op {
                    "==" => "e",
                    "!=" => "ne",
                    "<" => "l",
                    ">" => "g",
                    "<=" => "le",
                    ">=" => "ge",
                    _ => "e",
                };
                out.push_str(&format!("    set{} al\n", cc));
                out.push_str("    movzx rax, al\n");
                self.emit_store(out, &quad.result, "rax");
                self.record_temp_type(&quad.result, Type::Bool);
            }

            // ---- Logical AND / OR ----
            // Note: these do NOT short-circuit. Both operands are always
            // evaluated. That is a documented simplification of this
            // educational compiler (see docs/09-codegen-implementation.md).
            "&&" | "||" => {
                self.emit_load(out, &quad.arg1, "rax");
                self.emit_load(out, &quad.arg2, "rcx");
                if op == "&&" {
                    out.push_str("    and rax, rcx\n");
                } else {
                    out.push_str("    or rax, rcx\n");
                }
                self.emit_store(out, &quad.result, "rax");
                self.record_temp_type(&quad.result, Type::Bool);
            }

            // ---- Logical NOT ----
            "!" => {
                self.emit_load(out, &quad.arg1, "rax");
                out.push_str("    xor rax, 1\n");
                self.emit_store(out, &quad.result, "rax");
                self.record_temp_type(&quad.result, Type::Bool);
            }

            // ---- Assignment ----
            "=" => {
                self.emit_load(out, &quad.arg1, "rax");
                self.emit_store(out, &quad.result, "rax");
                // The destination takes the type of the source.
                let source_type = self.type_of(&quad.arg1);
                self.record_temp_type(&quad.result, source_type);
            }

            // ---- Print ----
            "PRINT" => {
                let ty = self.type_of(&quad.arg1);
                self.emit_load(out, &quad.arg1, "rdi");
                let routine = match ty {
                    Type::String => "neko_print_str",
                    Type::Bool => "neko_print_bool",
                    _ => "neko_print_int",
                };
                out.push_str(&format!("    call {}\n", routine));
            }

            // ---- Control flow ----
            "GOTO" => {
                out.push_str(&format!("    jmp {}\n", quad.result));
            }
            "GOTOF" => {
                self.emit_load(out, &quad.arg1, "rax");
                out.push_str("    test rax, rax\n");
                out.push_str(&format!("    jz {}\n", quad.result));
            }
            "GOTOV" => {
                self.emit_load(out, &quad.arg1, "rax");
                out.push_str("    test rax, rax\n");
                out.push_str(&format!("    jnz {}\n", quad.result));
            }
            "LABEL" => {
                out.push_str(&format!("{}:\n", quad.result));
            }

            // Unknown operator: emit a comment so the problem is visible
            // instead of silently producing wrong code.
            other => {
                out.push_str(&format!("    ; ERROR: unknown operator '{}'\n", other));
            }
        }
    }

    // ========================================================================
    // Load / store helpers
    // ========================================================================

    /// Emit the instructions that put an operand's value into `reg`.
    fn emit_load(&self, out: &mut String, operand_text: &str, reg: &str) {
        match self.classify(operand_text) {
            Operand::IntLiteral(n) => {
                out.push_str(&format!("    mov {}, {}\n", reg, n));
            }
            Operand::BoolLiteral(b) => {
                out.push_str(&format!("    mov {}, {}\n", reg, if b { 1 } else { 0 }));
            }
            Operand::StrLiteral(text) => {
                // A string literal's value IS its address. The literal was
                // already declared in the symbol-discovery pass, so we
                // only need to look its label up.
                let label = self
                    .symbols
                    .lookup_string_literal(&text)
                    .unwrap_or_else(|| "str_unknown".to_string());
                out.push_str(&format!("    lea {}, [rel {}]\n", reg, label));
            }
            Operand::Variable(name) => {
                let label = self
                    .symbols
                    .lookup(&name)
                    .map(|s| s.label())
                    .unwrap_or_else(|| format!("var_{}", name));
                out.push_str(&format!("    mov {}, [{}]\n", reg, label));
            }
            Operand::Temporary(name) => {
                out.push_str(&format!("    mov {}, [{}]\n", reg, name));
            }
            Operand::Label(name) => {
                out.push_str(&format!("    lea {}, [rel {}]\n", reg, name));
            }
            Operand::Unused => {
                out.push_str(&format!("    xor {}, {}\n", reg, reg));
            }
        }
    }

    /// Emit the instructions that store `reg` into a destination.
    fn emit_store(&self, out: &mut String, destination: &str, reg: &str) {
        match self.classify(destination) {
            Operand::Variable(name) => {
                let label = self
                    .symbols
                    .lookup(&name)
                    .map(|s| s.label())
                    .unwrap_or_else(|| format!("var_{}", name));
                out.push_str(&format!("    mov [{}], {}\n", label, reg));
            }
            Operand::Temporary(name) => {
                out.push_str(&format!("    mov [{}], {}\n", name, reg));
            }
            _ => {
                out.push_str(&format!(
                    "    ; ERROR: cannot store into '{}'\n",
                    destination
                ));
            }
        }
    }

    // ========================================================================
    // Type tracking
    // ========================================================================

    /// The static type of an operand.
    fn type_of(&self, operand_text: &str) -> Type {
        match self.classify(operand_text) {
            Operand::IntLiteral(_) => Type::Int,
            Operand::BoolLiteral(_) => Type::Bool,
            Operand::StrLiteral(_) => Type::String,
            Operand::Variable(name) => self
                .var_types
                .get(&name)
                .copied()
                .or_else(|| {
                    self.symbols
                        .lookup(&name)
                        .and_then(|s| s.ty)
                })
                .unwrap_or(Type::Int),
            Operand::Temporary(name) => {
                self.temp_types.get(&name).copied().unwrap_or(Type::Int)
            }
            Operand::Label(_) | Operand::Unused => Type::Int,
        }
    }

    /// Record the type of a temporary produced by an operation.
    fn record_temp_type(&mut self, name: &str, ty: Type) {
        if is_temporary_name(name) {
            self.temp_types.insert(name.to_string(), ty);
        }
    }

    // ========================================================================
    // Rendering
    // ========================================================================

    /// Put the whole program together.
    fn render(&self, body: String) -> String {
        let mut out = String::new();

        out.push_str("; ================================================================\n");
        out.push_str("; Neko — programa compilado\n");
        out.push_str("; Generado por neko-compiler\n");
        out.push_str(";\n");
        out.push_str("; Objetivo : Linux x86-64, sintaxis NASM\n");
        out.push_str("; Ensamblar: nasm -f elf64 program.asm -o program.o\n");
        out.push_str("; Enlazar  : ld program.o -o program\n");
        out.push_str("; Ejecutar : ./program\n");
        out.push_str("; ================================================================\n");
        out.push('\n');

        // ---- .data ----
        out.push_str("section .data\n");
        out.push_str("\n    ; --- textos del runtime ---\n");
        out.push_str("    str_true:      db \"true\", 0\n");
        out.push_str("    str_false:     db \"false\", 0\n");
        let strings = self.symbols.data_strings();
        if !strings.is_empty() {
            out.push_str("\n    ; --- literales de cadena del programa ---\n");
            for (label, text) in strings {
                // The trailing 0 is the NUL terminator. Every string in
                // Neko is NUL-terminated so `neko_print_str` can find
                // its length by scanning for it.
                out.push_str(&format!(
                    "    {:<14} db {}, 0\n",
                    format!("{}:", label),
                    nasm_string_literal(text)
                ));
            }
        }
        out.push('\n');

        // ---- .bss ----
        out.push_str("section .bss\n");
        out.push_str("\n    ; --- runtime: entrada del teclado y búferes ---\n");
        for symbol in self.symbols.bss_symbols() {
            if symbol.kind != SymbolKind::Runtime {
                continue;
            }
            // `numbuf` needs an `equ` for its end so the runtime can
            // build the decimal representation backwards from there.
            if symbol.name == "numbuf" {
                out.push_str(&format!(
                    "    {:<14} resb {}\n",
                    format!("{}:", symbol.name),
                    symbol.size
                ));
                out.push_str(&format!(
                    "    {:<14} equ {} + {}\n",
                    format!("{}:", "numbuf_end"),
                    symbol.name,
                    symbol.size
                ));
                continue;
            }
            // The directive must match the unit: `resb` for byte
            // buffers, `resq` for 8-byte slots.
            let directive = match symbol.unit {
                Unit::Bytes => "resb",
                Unit::Quadwords => "resq",
            };
            let count = match symbol.unit {
                Unit::Bytes => symbol.size,
                Unit::Quadwords => symbol.size / 8,
            };
            out.push_str(&format!(
                "    {:<14} {} {}\n",
                format!("{}:", symbol.name),
                directive,
                count
            ));
        }

        let bss_symbols = self.symbols.bss_symbols();
        let mut any_variable = false;
        for symbol in &bss_symbols {
            if symbol.kind == SymbolKind::Variable {
                if !any_variable {
                    out.push_str("\n    ; --- variables del usuario ---\n");
                    any_variable = true;
                }
                out.push_str(&format!(
                    "    {:<14} resq {}\n",
                    format!("{}:", symbol.label()),
                    symbol.size / 8
                ));
            }
        }
        let mut any_temp = false;
        for symbol in self.symbols.sorted_temporaries() {
            if !any_temp {
                out.push_str("\n    ; --- temporales del compilador ---\n");
                any_temp = true;
            }
            out.push_str(&format!(
                "    {:<14} resq {}\n",
                format!("{}:", symbol.label()),
                symbol.size / 8
            ));
        }
        out.push('\n');

        // ---- .text ----
        out.push_str("section .text\n");
        out.push_str("global _start\n");
        out.push('\n');
        out.push_str(&format!("STACK_SIZE equ {}\n", STACK_SIZE));
        out.push('\n');
        out.push_str("_start:\n");
        out.push_str("    ; Reserva del stack. Neko tiene una sola funcion\n");
        out.push_str("    ; (fn neko) sin parametros, asi que no hay frames:\n");
        out.push_str("    ; todo el programa corre en un unico frame.\n");
        out.push_str("    mov  rbp, rsp\n");
        out.push_str(&format!("    sub  rsp, {}\n", STACK_SIZE));
        out.push('\n');
        out.push_str("    ; ---- cuerpo del programa (generado desde los cuadruplos) ----\n");
        out.push_str(&body);
        out.push('\n');
        out.push_str("    ; ---- salida: exit(0) ----\n");
        out.push_str("    mov  rax, 60\n");
        out.push_str("    xor  rdi, rdi\n");
        out.push_str("    syscall\n");
        out.push('\n');

        // ---- runtime routines ----
        out.push_str("; ================================================================\n");
        out.push_str("; Rutinas del runtime de Neko\n");
        out.push_str("; Las interrupciones de entrada y salida del sistema:\n");
        out.push_str(";   read  (rax=0)  -> fd 0 = teclado\n");
        out.push_str(";   write (rax=1)  -> fd 1 = pantalla\n");
        out.push_str(";   exit  (rax=60) -> fin del programa\n");
        out.push_str("; ================================================================\n");
        for routine in runtime::ALL_ROUTINES {
            out.push_str(routine);
            out.push('\n');
        }

        out
    }

    /// The symbol table, for writing `output/symbol_table.txt`.
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbols
    }
}

/// Is this the name of a compiler temporary (`T1`, `T2`, ...)?
fn is_temporary_name(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some('T') => {}
        _ => return false,
    }
    let rest: String = chars.collect();
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

/// Is this the name of a jump label (`L1`, `L2`, ...)?
fn is_label_name(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some('L') => {}
        _ => return false,
    }
    let rest: String = chars.collect();
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

/// Render a string as a NASM `db` operand, escaping what needs escaping.
///
/// Neko string literals can only contain alphanumerics, spaces and
/// underscores (the lexer rejects anything else), but we escape
/// defensively anyway so the generated assembly is always valid.
fn nasm_string_literal(text: &str) -> String {
    let mut out = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
