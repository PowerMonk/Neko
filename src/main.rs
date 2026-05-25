// Declare the `lexer` submodule.
// Rust looks for `src/lexer/mod.rs` (or `src/lexer.rs`).
// In TS: `import * as lexer from './lexer'` — but here we're declaring
// the module exists and telling the compiler to include it.
mod lexer;

// `use crate::lexer::Lexer` — imports the Lexer struct from our module.
// `crate` refers to the root of this project.
// In TS: `import { Lexer } from './lexer/index'`
use crate::lexer::Lexer;

use std::env;  // Access command-line arguments
use std::fs;   // File I/O (read_to_string, write, create_dir_all)
use std::io;   // I/O error type (used in the Result return type)

// `io::Result<()>` is shorthand for `Result<(), io::Error>`.
// `()` (unit type) = "no meaningful value" — like `void` in TS.
// The `?` operator inside the function can return `Err(...)` early.
fn main() -> io::Result<()> {
    // `env::args()` returns an iterator over CLI arguments.
    // `.nth(1)` gets the element at index 1 (0 = program name, 1 = first arg).
    // Returns `Option<String>` — could be `Some("myfile.neko")` or `None`.
    // `.unwrap_or_else(|| default)` — if `None`, return the provided default.
    // In TS: `const inputPath = process.argv[2] ?? "examples/basic.neko"`
    let input_path = env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("examples/basic.neko"));

    // Read the source file into a String.
    // `?` operator: if `read_to_string` returns `Err`, exit `main()` early
    // with that error. Like `try { await readFile(path) } catch (e) { return e; }`.
    let source = fs::read_to_string(&input_path)?;
    let lexer = Lexer::new(source);
    let analysis = lexer.lex();

    // Write the three output files to the "output" directory.
    // `?` propagates any I/O errors (e.g., disk full, permission denied).
    analysis.write_outputs("output")?;

    // `{}` is Rust's placeholder for formatted output.
    println!(
        "Lexical analysis completed: {} token(s), {} symbol(s), {} error(s).",
        analysis.tokens.len(),
        analysis.symbols.entries().len(),
        analysis.errors.len()
    );

    Ok(())  // `Ok(())` means "success, returning nothing" — like `return;` in TS
}
