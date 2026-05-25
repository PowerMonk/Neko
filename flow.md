# Neko Lexer — Code Flow

## 1. Overview

This is the **lexer** (tokenizer) for the Neko programming language. It reads raw source code and produces tokens, a symbol table, and any lexical errors. This is **just the lexer** — no parser, no compiler — a focused educational project.

## 2. Project Structure

```
Neko/
├── Cargo.toml              ← Rust project config (no external dependencies)
├── examples/
│   └── basic.neko          ← Sample Neko source file (default input)
├── output/                 ← Generated output directory
│   ├── tokens.txt          ← Token stream output
│   ├── symbols.txt         ← Identifier table output
│   └── errors.txt          ← Error report output
└── src/
    ├── main.rs             ← Entry point: reads file, runs lexer, writes output
    └── lexer/
        ├── mod.rs          ← Module declarations & re-exports
        ├── token.rs        ← Data types: TokenKind enum, Token struct, LexicalError
        ├── alphabet.rs     ← Character classifiers (is this a digit? letter? operator?)
        ├── automata.rs     ← DFA recognizers (match keywords, identifiers, strings, etc.)
        ├── symbol_table.rs ← SymbolTable: deduplicated identifier storage
        └── lexer.rs        ← Lexer orchestrator: drives the scanning loop
```

## 3. Module Dependency Graph

```
main.rs
  └── lexer crate
        ├── token.rs        ← data types (standalone — no deps)
        ├── alphabet.rs     ← char predicates (standalone — no deps)
        ├── automata.rs     ← DFA matching (depends on alphabet + token)
        ├── symbol_table.rs ← identifier storage (standalone — no deps)
        └── lexer.rs        ← orchestrator (depends on all four above)
```

Dependency direction: `lexer.rs` imports from all others. `automata.rs` imports from `alphabet.rs` and `token.rs`. `token.rs`, `alphabet.rs`, and `symbol_table.rs` are leaf modules with no internal dependencies.

## 4. Pipeline Flow (Step by Step)

### Step 1: Entry — `main.rs`

```
CLI argument (path to .neko file) or default "examples/basic.neko"
  → fs::read_to_string() reads the file into a String
  → Lexer::new(source) creates the lexer, normalizes line endings,
    converts source to Vec<char> for O(1) index access
```

### Step 2: Scan Loop — `Lexer::lex()` → `scan_tokens()`

```
while peek() returns Some(char):

  1. Invalid char?           → record error, advance
  2. Whitespace?            → consume all whitespace, continue
  3. '#' (comment)?         → scan_comment()
  4. '"' (string literal)?  → scan_string_literal()
  5. '_' (wildcard)?        → scan_wildcard()
  6. Alphabetic?            → scan_identifier_or_keyword()
       ├─ try_keyword()     → emit keyword token (e.g. FN, NEKO, MATCH)
       └─ recognize_identifier() → emit IDENTIFIER + add to symbol table
  7. Digit?                 → scan_integer()       → emit INTEGER
  8. Operator char?         → scan_operator_or_delimiter()
       ├─ recognize_operator()  → emit operator token (e.g. PLUS, EQUAL_EQUAL, FAT_ARROW)
       └─ recognize_delimiter() → emit delimiter token (e.g. LEFT_PAREN, RIGHT_BRACE)
  9. Delimiter char?        → scan_operator_or_delimiter() (same path as #8)
  10. Nothing matched       → record "unexpected symbol" error, advance
```

Each `scan_*` method:
1. Records the start position (index, line, column)
2. Calls the corresponding `recognize_*()` function from `automata.rs`
3. On match: collects the lexeme substring, advances the index, emits a token
4. On failure: records an error at the current position

### Step 3: Recognition — `automata.rs`

Each `recognize_*()` is a hand-written Deterministic Finite Automaton (DFA):

| Recognizer | Signature | Description |
|---|---|---|
| `recognize_keyword_*` | `(&[char], usize) → Option<usize>` | Checks specific chars at fixed offsets + word boundary |
| `recognize_identifier` | `(&[char], usize) → Option<usize>` | Greedy `[a-zA-Z][a-zA-Z0-9_]*` |
| `recognize_integer` | `(&[char], usize) → Option<usize>` | Greedy `[0-9]+` |
| `recognize_string` | `(&[char], usize) → StringRecognition` | DFA returning matched/unterminated/invalid-char |
| `recognize_comment` | `(&[char], usize) → Option<usize>` | `#` to end-of-line |
| `recognize_wildcard` | `(&[char], usize) → Option<usize>` | Exactly `_` |
| `recognize_operator` | `(&[char], usize) → Option<(TokenKind, usize)>` | Multi-char (`==`, `!=`, `<=`, `>=`, `&&`, `\|\|`, `=>`) then single-char |
| `recognize_delimiter` | `(&[char], usize) → Option<(TokenKind, usize)>` | Single-char: `( ) { } ,` |

### Step 4: Output — `LexicalAnalysis::write_outputs()`

```
LexicalAnalysis { tokens, symbols, errors }
  → write_outputs("output")
      ├── output/tokens.txt   ← "KIND -> lexeme" per line
      ├── output/symbols.txt  ← one identifier per line (insertion order)
      └── output/errors.txt   ← "line:col -> message" per line
```

## 5. Visual Pipeline

```
 Source File
    │
    ▼
 ┌─────────────────┐
 │  main.rs         │
 │  Read file       │
 │  Call Lexer::new │
 └────────┬─────────┘
          │  String source
          ▼
 ┌─────────────────┐
 │  Lexer::new()    │
 │  Normalize \r\n  │
 │  → Vec<char>     │
 └────────┬─────────┘
          │  Vec<char>
          ▼
 ┌─────────────────────┐
 │  Lexer::lex()       │
 │  → scan_tokens()    │
 │                     │
 │  ┌───────────────┐  │
 │  │  peek() loop   │  │
 │  │  ┌───────┐    │  │
 │  │  │ char? │────┼──┼──► alphabet.rs (predicates)
 │  │  └───┬───┘    │  │       ▲
 │  │      │        │  │       │
 │  │      ▼        │  │  automata.rs (recognizers)
 │  │  scan_*() ────┼──┼───────┘
 │  │      │        │  │
 │  │      ▼        │  │
 │  │  emit_token() │  │
 │  │  record_err() │  │
 │  └───────────────┘  │
 └────────┬─────────────┘
          │  tokens, symbols, errors
          ▼
 ┌──────────────────────┐
 │  LexicalAnalysis     │
 │  write_outputs()     │
 └────────┬─────────────┘
          │
     ┌────┼────┐
     ▼    ▼    ▼
  tokens  syms  errors
   .txt   .txt   .txt
```

## 6. Rust Concepts for TypeScript Developers

| TypeScript | Rust | Notes |
|---|---|---|
| `import { X } from './file'` | `use crate::mod::X;` | `mod file;` declares the submodule |
| `export class Foo {}` | `pub struct Foo {}` + `impl Foo {}` | Data (struct) and methods (impl) are separate |
| `enum Foo { A, B }` | `enum Foo { A, B }` | Rust enums are full sum types — variants carry data |
| `T \| null` | `Option<T>` | `None` or `Some(value)` — must be explicitly handled |
| `throw Error` / `try/catch` | `Result<T, E>` / `?` operator | No exceptions — errors are return values |
| `switch(x) { case A: }` | `match x { A => ... }` | Exhaustive — compiler checks all cases covered |
| `@decorator` | `#[derive(Debug, Clone)]` | Auto-implements common traits |
| `arr[i]` | `arr.get(i)` (safe) or `arr[i]` (panic on OOB) | `.get()` returns `Option<&T>` |
| `string` | `String` (owned) or `&str` (borrowed) | Two types, not one — ownership matters |
| `void` | `()` | Unit type |
| `as` cast | `as` keyword | Similar syntax |
| `private` field | no `pub` = private | Everything is private by default |

## 7. Key Design Decisions

1. **`Vec<char>` over `&str`** — O(1) indexed access without UTF-8 byte navigation. Neko uses only ASCII, so this is safe and pedagogically simpler.

2. **Hand-written DFA over regex** — Each keyword and token has an explicit character-by-character recognizer. Verbose but educational — visibly demonstrates DFA mechanics.

3. **Error resilience** — The lexer never panics. Invalid input produces `LexicalError` entries and scanning continues. String errors have dedicated recovery to prevent cascading failures.

4. **Ownership by design** — `Lexer::lex()` takes `mut self` (consumes the lexer). After lexing, the only way to access results is through the returned `LexicalAnalysis`. This prevents reusing a spent lexer.

5. **Keyword boundary checking** — `keyword_boundary()` ensures that "neko" doesn't match inside "nekocat". Keywords must be followed by a non-identifier character (or EOF).

6. **No external dependencies** — The entire project uses only the Rust standard library (`std::env`, `std::fs`, `std::io`, `std::path`). Zero `crates.io` dependencies.
