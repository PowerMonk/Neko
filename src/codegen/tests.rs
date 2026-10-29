// ============================================================================
// Code generation tests
//
// Strategy: build quadruple lists by hand, feed them to the Assembler,
// and assert on the generated assembly text. We test the translation of
// each quadruple shape in isolation, plus the structure of the whole
// file (sections, runtime, prologue).
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::assembler::Assembler;
    use super::super::runner::to_wsl_path;
    use super::super::symbol_table::{Section, SymbolKind, SymbolTable};
    use crate::icg::quadruple::Quadruple;
    use crate::semantic::Type;
    use std::collections::HashMap;
    use std::path::Path;

    /// Assemble a list of quadruples with no user variables.
    fn assemble(quads: Vec<Quadruple>) -> String {
        assemble_with(quads, HashMap::new())
    }

    /// Assemble a list of quadruples, with a known set of user variables
    /// and their types.
    fn assemble_with(quads: Vec<Quadruple>, vars: HashMap<String, Type>) -> String {
        let mut assembler = Assembler::new(vars);
        assembler.assemble(&quads)
    }

    fn q(op: &str, a: &str, b: &str, r: &str) -> Quadruple {
        Quadruple::new(op, a, b, r)
    }

    // ============================================================
    // Symbol table
    // ============================================================

    #[test]
    fn runtime_symbols_are_preloaded() {
        let table = SymbolTable::with_runtime();
        // The teacher's notes require the keyboard input buffer to be
        // in the symbol table.
        let stdin = table.lookup("stdin").expect("stdin must exist");
        assert_eq!(stdin.kind, SymbolKind::Runtime);
        assert_eq!(stdin.section, Section::Bss);
        assert_eq!(stdin.size, 256);
        // And the scratch buffer used to print integers.
        assert!(table.lookup("numbuf").is_some());
        assert!(table.lookup("stdin_len").is_some());
        assert!(table.lookup("stdout").is_some());
    }

    #[test]
    fn variables_and_temporaries_get_distinct_offsets() {
        let mut table = SymbolTable::with_runtime();
        table.declare_variable("x", Type::Int);
        table.declare_variable("y", Type::String);
        table.declare_temporary("T1");

        let x = table.lookup("x").unwrap();
        let y = table.lookup("y").unwrap();
        let t1 = table.lookup("T1").unwrap();
        // All in .bss, all with distinct offsets.
        assert_eq!(x.section, Section::Bss);
        assert_ne!(x.offset, y.offset);
        assert_ne!(x.offset, t1.offset);
        assert_eq!(x.size, 8);
    }

    #[test]
    fn identical_string_literals_share_one_symbol() {
        let mut table = SymbolTable::with_runtime();
        let a = table.declare_string("hola");
        let b = table.declare_string("hola");
        let c = table.declare_string("adios");
        assert_eq!(a, b, "identical literals must share a symbol");
        assert_ne!(a, c);
    }

    #[test]
    fn variable_labels_are_prefixed() {
        let mut table = SymbolTable::with_runtime();
        table.declare_variable("score", Type::Int);
        assert_eq!(table.lookup("score").unwrap().label(), "var_score");
    }

    // ============================================================
    // Translation of individual quadruples
    // ============================================================

    #[test]
    fn addition_uses_add_instruction() {
        let asm = assemble(vec![q("+", "a", "b", "T1")]);
        assert!(asm.contains("add rax, rcx"));
        assert!(asm.contains("mov [T1], rax"));
    }

    #[test]
    fn subtraction_uses_sub_instruction() {
        let asm = assemble(vec![q("-", "a", "b", "T1")]);
        assert!(asm.contains("sub rax, rcx"));
    }

    #[test]
    fn multiplication_uses_imul() {
        let asm = assemble(vec![q("*", "a", "b", "T1")]);
        assert!(asm.contains("imul rax, rcx"));
    }

    #[test]
    fn division_uses_signed_division_and_keeps_quotient() {
        let asm = assemble(vec![q("/", "a", "b", "T1")]);
        assert!(asm.contains("cqo"));
        assert!(asm.contains("idiv rcx"));
        // The quotient is in rax.
        assert!(asm.contains("mov [T1], rax"));
    }

    #[test]
    fn modulo_keeps_the_remainder() {
        let asm = assemble(vec![q("%", "a", "b", "T1")]);
        assert!(asm.contains("idiv rcx"));
        // The remainder is in rdx, not rax.
        assert!(asm.contains("mov [T1], rdx"));
    }

    #[test]
    fn unary_minus_uses_neg_not_sub() {
        // The unary minus is the one with arg2 == "_". It must NOT be
        // translated as a binary subtraction.
        let asm = assemble(vec![q("-", "a", "_", "T1")]);
        assert!(asm.contains("neg rax"));
        assert!(!asm.contains("sub rax, rcx"));
    }

    #[test]
    fn comparisons_use_setcc_and_movzx() {
        let cases = [
            ("==", "sete"),
            ("!=", "setne"),
            ("<", "setl"),
            (">", "setg"),
            ("<=", "setle"),
            (">=", "setge"),
        ];
        for (op, setcc) in cases {
            let asm = assemble(vec![q(op, "a", "b", "T1")]);
            assert!(asm.contains("cmp rax, rcx"), "op {}", op);
            assert!(asm.contains(&format!("{} al", setcc)), "op {}", op);
            assert!(asm.contains("movzx rax, al"), "op {}", op);
        }
    }

    #[test]
    fn logical_and_uses_and_instruction() {
        let asm = assemble(vec![q("&&", "a", "b", "T1")]);
        assert!(asm.contains("and rax, rcx"));
    }

    #[test]
    fn logical_or_uses_or_instruction() {
        let asm = assemble(vec![q("||", "a", "b", "T1")]);
        assert!(asm.contains("or rax, rcx"));
    }

    #[test]
    fn logical_not_uses_xor_with_one() {
        let asm = assemble(vec![q("!", "a", "_", "T1")]);
        assert!(asm.contains("xor rax, 1"));
    }

    #[test]
    fn assignment_stores_into_the_destination() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Type::Int);
        let asm = assemble_with(vec![q("=", "42", "_", "x")], vars);
        assert!(asm.contains("mov rax, 42"));
        assert!(asm.contains("mov [var_x], rax"));
    }

    #[test]
    fn integer_literal_is_moved_immediately() {
        let asm = assemble(vec![q("+", "42", "7", "T1")]);
        assert!(asm.contains("mov rax, 42"));
        assert!(asm.contains("mov rcx, 7"));
    }

    #[test]
    fn string_literal_is_loaded_by_address() {
        let asm = assemble(vec![q("=", "\"Hola\"", "_", "x")]);
        // A string's value IS its address, so we use `lea`.
        assert!(asm.contains("lea rax, [rel str_0]"));
    }

    // ============================================================
    // Printing: the routine depends on the operand's type
    // ============================================================

    #[test]
    fn print_int_uses_print_int_routine() {
        let mut vars = HashMap::new();
        vars.insert("n".to_string(), Type::Int);
        let asm = assemble_with(vec![q("PRINT", "n", "_", "_")], vars);
        assert!(asm.contains("call neko_print_int"));
    }

    #[test]
    fn print_string_uses_print_str_routine() {
        let mut vars = HashMap::new();
        vars.insert("s".to_string(), Type::String);
        let asm = assemble_with(vec![q("PRINT", "s", "_", "_")], vars);
        assert!(asm.contains("call neko_print_str"));
    }

    #[test]
    fn print_bool_uses_print_bool_routine() {
        let mut vars = HashMap::new();
        vars.insert("b".to_string(), Type::Bool);
        let asm = assemble_with(vec![q("PRINT", "b", "_", "_")], vars);
        assert!(asm.contains("call neko_print_bool"));
    }

    #[test]
    fn print_literal_string_uses_print_str_routine() {
        let asm = assemble(vec![q("PRINT", "\"Hola\"", "_", "_")]);
        assert!(asm.contains("call neko_print_str"));
    }

    // ============================================================
    // Control flow
    // ============================================================

    #[test]
    fn goto_becomes_jmp() {
        let asm = assemble(vec![q("GOTO", "_", "_", "L1")]);
        assert!(asm.contains("jmp L1"));
    }

    #[test]
    fn gotof_becomes_test_and_jz() {
        let asm = assemble(vec![q("GOTOF", "c", "_", "L1")]);
        assert!(asm.contains("test rax, rax"));
        assert!(asm.contains("jz L1"));
    }

    #[test]
    fn gotov_becomes_test_and_jnz() {
        let asm = assemble(vec![q("GOTOV", "c", "_", "L1")]);
        assert!(asm.contains("test rax, rax"));
        assert!(asm.contains("jnz L1"));
    }

    #[test]
    fn label_becomes_a_label() {
        let asm = assemble(vec![q("LABEL", "_", "_", "L1")]);
        assert!(asm.contains("L1:"));
    }

    // ============================================================
    // File structure
    // ============================================================

    #[test]
    fn file_has_the_three_sections() {
        let asm = assemble(vec![]);
        assert!(asm.contains("section .data"));
        assert!(asm.contains("section .bss"));
        assert!(asm.contains("section .text"));
    }

    #[test]
    fn entry_point_is_global_and_exits_cleanly() {
        let asm = assemble(vec![]);
        assert!(asm.contains("global _start"));
        assert!(asm.contains("_start:"));
        // exit(0) at the end: rax = 60, rdi = 0, syscall.
        assert!(asm.contains("mov  rax, 60"));
        assert!(asm.contains("xor  rdi, rdi"));
        assert!(asm.contains("syscall"));
    }

    #[test]
    fn stack_is_reserved_with_the_declared_size() {
        let asm = assemble(vec![]);
        assert!(asm.contains("STACK_SIZE equ 4096"));
        assert!(asm.contains("sub  rsp, 4096"));
    }

    #[test]
    fn runtime_routines_are_embedded() {
        let asm = assemble(vec![]);
        assert!(asm.contains("neko_print_str:"));
        assert!(asm.contains("neko_print_int:"));
        assert!(asm.contains("neko_print_bool:"));
        assert!(asm.contains("neko_read_line:"));
    }

    #[test]
    fn runtime_uses_the_linux_syscalls() {
        let asm = assemble(vec![]);
        // write: rax = 1
        assert!(asm.contains("mov  rax, 1"));
        // read: rax = 0
        assert!(asm.contains("mov  rax, 0"));
        // stdout = fd 1, stdin = fd 0
        assert!(asm.contains("mov  rdi, 1"));
        assert!(asm.contains("xor  rdi, rdi"));
    }

    #[test]
    fn string_literals_are_nul_terminated() {
        let asm = assemble(vec![q("=", "\"Hola\"", "_", "x")]);
        // Every string must end with ", 0" so the runtime can find its
        // length by scanning for the terminator.
        assert!(asm.contains("db \"Hola\", 0"));
    }

    #[test]
    fn runtime_texts_are_emitted() {
        let asm = assemble(vec![]);
        assert!(asm.contains("db \"true\", 0"));
        assert!(asm.contains("db \"false\", 0"));
    }

    #[test]
    fn keyboard_buffer_is_in_the_bss_section() {
        let asm = assemble(vec![]);
        assert!(asm.contains("stdin:"));
        assert!(asm.contains("resb 256"));
    }

    // ============================================================
    // WSL path translation
    // ============================================================

    #[test]
    fn windows_path_becomes_a_mnt_path() {
        let translated = to_wsl_path(Path::new(r"C:\Users\aroml\Neko\output"));
        assert_eq!(translated, "/mnt/c/Users/aroml/Neko/output");
    }

    #[test]
    fn linux_path_is_left_alone() {
        let translated = to_wsl_path(Path::new("/home/user/neko"));
        assert_eq!(translated, "/home/user/neko");
    }

    #[test]
    fn drive_letter_is_lowercased() {
        let translated = to_wsl_path(Path::new(r"D:\work\out"));
        assert_eq!(translated, "/mnt/d/work/out");
    }
}
