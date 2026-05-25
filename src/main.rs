mod lexer;

use crate::lexer::Lexer;

use std::env;
use std::fs;
use std::io;

fn main() -> io::Result<()> {
    let input_path = env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("examples/basic.neko"));

    let source = fs::read_to_string(&input_path)?;
    let lexer = Lexer::new(source);
    let analysis = lexer.lex();

    analysis.write_outputs("output")?;

    println!(
        "Lexical analysis completed: {} token(s), {} symbol(s), {} error(s).",
        analysis.tokens.len(),
        analysis.symbols.entries().len(),
        analysis.errors.len()
    );

    Ok(())
}
