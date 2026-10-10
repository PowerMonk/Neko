// Declare the compiler submodules.
mod icg;
mod lexer;
mod parser;
mod semantic;

use crate::icg::ICGenerator;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::semantic::SemanticAnalyzer;

use std::env;
use std::fs;
use std::io;

fn main() -> io::Result<()> {
    let input_path = env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("examples/basic.neko"));

    // ---- Phase 1: Lexical analysis ----
    let source = fs::read_to_string(&input_path)?;
    let lexer = Lexer::new(source);
    let lex_result = lexer.lex();
    lex_result.write_outputs("output")?;

    // ---- Phase 2: Syntactic analysis (only if lexer is clean) ----
    let (syn_errors, syn_ok, parse_result_opt) = if lex_result.errors.is_empty() {
        let parser = Parser::new(lex_result.tokens.clone());
        let syn_result = parser.parse();
        let count = syn_result.errors.len();
        syn_result.write_outputs("output")?;
        (count, true, Some(syn_result))
    } else {
        std::fs::write("output/parse_tree.txt", "")?;
        std::fs::write(
            "output/parse_errors.txt",
            "(skipped: lexical errors present)\n",
        )?;
        (0usize, false, None)
    };

    // ---- Phase 3: Semantic analysis (only if both prior phases are clean) ----
    let sem_errors = if let (true, Some(parse_result)) = (syn_ok, parse_result_opt.as_ref()) {
        if parse_result.errors.is_empty() {
            let analyzer = SemanticAnalyzer::new(lex_result.tokens.clone());
            let sem_result = analyzer.analyze(&parse_result.program);
            let count = sem_result.errors.len();
            sem_result.write_output("output")?;
            count
        } else {
            std::fs::write(
                "output/semantic_errors.txt",
                "(skipped: syntactic errors present)\n",
            )?;
            0
        }
    } else {
        std::fs::write(
            "output/semantic_errors.txt",
            "(skipped: lexical errors present)\n",
        )?;
        0
    };

    // ---- Phase 4: Intermediate code generation (only if all prior phases clean) ----
    let (quad_count, triple_count) =
        if let (true, Some(parse_result)) = (syn_ok, parse_result_opt.as_ref()) {
            if parse_result.errors.is_empty() && sem_errors == 0 {
                let generator = ICGenerator::new();
                let generated = generator.generate(&parse_result.program);
                std::fs::create_dir_all("output")?;
                std::fs::write("output/quadruples.txt", &generated.quadruples)?;
                std::fs::write("output/triples.txt", &generated.triples)?;
                (generated.quad_count, generated.triple_count)
            } else {
                std::fs::write("output/quadruples.txt", "(skipped: errors in prior phases)\n")?;
                std::fs::write("output/triples.txt", "(skipped: errors in prior phases)\n")?;
                (0, 0)
            }
        } else {
            std::fs::write("output/quadruples.txt", "(skipped: errors in prior phases)\n")?;
            std::fs::write("output/triples.txt", "(skipped: errors in prior phases)\n")?;
            (0, 0)
        };

    // ---- Terminal summary ----
    let tok_count = lex_result.tokens.len();
    let sym_count = lex_result.symbols.entries().len();
    let lex_errors = lex_result.errors.len();

    if syn_ok && syn_errors == 0 && sem_errors == 0 {
        println!(
            "OK — {} tokens, {} symbols, parse tree built with 0 syntactic errors, semantic analysis: 0 errors, intermediate code: {} quadruples, {} triples.",
            tok_count, sym_count, quad_count, triple_count
        );
    } else {
        println!(
            "Errors found — lexical: {}, syntactic: {}, semantic: {}. See output/errors.txt, output/parse_errors.txt, output/semantic_errors.txt.",
            lex_errors, syn_errors, sem_errors
        );
    }

    Ok(())
}
