// ============================================================================
// Neko compiler — pipeline principal.
//
//   fuente .neko
//        │
//        ├── Fase 1: análisis léxico      (src/lexer/)     → tokens, símbolos, errores léxicos
//        ├── Fase 2: análisis sintáctico   (src/parser/)    → árbol sintáctico, errores sintácticos
//        ├── Fase 3: análisis semántico    (src/semantic/)  → tabla de símbolos con tipos, errores semánticos
//        ├── Fase 4: código intermedio     (src/icg/)       → cuadruplos y triplos
//        └── Fase 5: código ensamblador    (src/codegen/)   → program.asm
//
// Cada fase sólo corre si todas las anteriores考评aron limpias: un
// error en una fase temprana hace que las siguientes se omitan (se
// escribe un texto marcador en su archivo de salida).
// ============================================================================

// Declare the compiler submodules.
mod codegen;
mod icg;
mod lexer;
mod parser;
mod semantic;

use crate::codegen::runner::{build_and_run, manual_instructions, RunOutcome};
use crate::codegen::Assembler;
use crate::icg::ICGenerator;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::semantic::SemanticAnalyzer;

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

fn main() -> io::Result<()> {
    // ---- Command line ----
    //   cargo run -- [archivo.neko] [--no-run]
    //
    // `--no-run` genera el .asm pero no lo ensambla ni lo ejecuta.
    let mut input_path: Option<String> = None;
    let mut should_run = true;
    for argument in env::args().skip(1) {
        if argument == "--no-run" {
            should_run = false;
        } else if input_path.is_none() {
            input_path = Some(argument);
        }
    }
    let input_path = input_path.unwrap_or_else(|| String::from("examples/basic.neko"));

    // The directory where every generated artefact is written.
    let output_dir = PathBuf::from("output");

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
    // `semantic_result` is kept so the code generator can use the symbol
    // table (name -> type) when it translates the quadruples.
    let (sem_errors, sem_result_opt) =
        if let (true, Some(parse_result)) = (syn_ok, parse_result_opt.as_ref()) {
            if parse_result.errors.is_empty() {
                let analyzer = SemanticAnalyzer::new(lex_result.tokens.clone());
                let sem_result = analyzer.analyze(&parse_result.program);
                let count = sem_result.errors.len();
                sem_result.write_output("output")?;
                (count, Some(sem_result))
            } else {
                std::fs::write(
                    "output/semantic_errors.txt",
                    "(skipped: syntactic errors present)\n",
                )?;
                (0usize, None)
            }
        } else {
            std::fs::write(
                "output/semantic_errors.txt",
                "(skipped: lexical errors present)\n",
            )?;
            (0usize, None)
        };

    // ---- Phase 4: Intermediate code generation ----
    // The structured quadruples are kept for the code generator.
    let (quad_count, triple_count, generated_opt) =
        if let (true, Some(parse_result)) = (syn_ok, parse_result_opt.as_ref()) {
            if parse_result.errors.is_empty() && sem_errors == 0 {
                let generator = ICGenerator::new();
                let generated = generator.generate(&parse_result.program);
                std::fs::create_dir_all("output")?;
                std::fs::write("output/quadruples.txt", &generated.quadruples)?;
                std::fs::write("output/triples.txt", &generated.triples)?;
                (
                    generated.quad_count,
                    generated.triple_count,
                    Some(generated),
                )
            } else {
                std::fs::write(
                    "output/quadruples.txt",
                    "(skipped: errors in prior phases)\n",
                )?;
                std::fs::write(
                    "output/triples.txt",
                    "(skipped: errors in prior phases)\n",
                )?;
                (0, 0, None)
            }
        } else {
            std::fs::write(
                "output/quadruples.txt",
                "(skipped: errors in prior phases)\n",
            )?;
            std::fs::write(
                "output/triples.txt",
                "(skipped: errors in prior phases)\n",
            )?;
            (0, 0, None)
        };

    // ---- Phase 5: Assembly code generation ----
    // The assembler needs the quadruples (what to do) AND the semantic
    // symbol table (where things live and what type they have).
    let mut assembly_written = false;
    if let (Some(generated), Some(sem_result)) = (generated_opt.as_ref(), sem_result_opt.as_ref()) {
        let mut assembler = Assembler::new(sem_result.symbols.clone());
        let assembly = assembler.assemble(&generated.quadruple_list);
        std::fs::write("output/program.asm", &assembly)?;
        std::fs::write("output/symbol_table.txt", assembler.symbol_table().render())?;
        assembly_written = true;
    } else {
        std::fs::write(
            "output/program.asm",
            "; (skipped: errors in prior phases)\n",
        )?;
        std::fs::write(
            "output/symbol_table.txt",
            "(skipped: errors in prior phases)\n",
        )?;
    }

    // ---- Terminal summary ----
    let tok_count = lex_result.tokens.len();
    let sym_count = lex_result.symbols.entries().len();
    let lex_errors = lex_result.errors.len();

    if syn_ok && syn_errors == 0 && sem_errors == 0 {
        println!(
            "OK — {} tokens, {} symbols, parse tree built with 0 syntactic errors, semantic analysis: 0 errors, intermediate code: {} quadruples, {} triples, assembly: output/program.asm.",
            tok_count, sym_count, quad_count, triple_count
        );
    } else {
        println!(
            "Errors found — lexical: {}, syntactic: {}, semantic: {}. See output/errors.txt, output/parse_errors.txt, output/semantic_errors.txt.",
            lex_errors, syn_errors, sem_errors
        );
    }

    // ---- Phase 6: assemble, link and run the generated program ----
    if assembly_written && should_run {
        println!("\n--- ejecutando el programa compilado ---");
        match build_and_run(&output_dir) {
            RunOutcome::Ran { output } => {
                print!("{}", output);
                if !output.ends_with('\n') {
                    println!();
                }
            }
            RunOutcome::Failed { reason } => {
                println!("No se pudo ejecutar el programa: {}", reason);
                println!("{}", manual_instructions(&output_dir));
            }
        }
    }

    Ok(())
}
