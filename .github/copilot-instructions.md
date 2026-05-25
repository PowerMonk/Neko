# Neko Compiler - Copilot Instructions

## Project Overview

Neko is a minimalistic compiled programming language written in Rust.

The current implementation phase focuses ONLY on lexical analysis.

The lexer must perform:

1. Character-level validation against the language alphabet.
2. Lexeme recognition using deterministic finite automata (DFA).
3. Symbol table generation.

---

# Architecture Rules

- Use modular Rust architecture.
- Keep lexer logic separated by responsibility.
- Each lexical category must have its own recognizer function.

Example:

- recognize_keyword_nyan()
- recognize_keyword_fn()
- recognize_identifier()
- recognize_integer()
- recognize_string()

Do NOT merge all keywords into a hashmap recognizer.
The project intentionally uses explicit DFA-like recognition for academic purposes.

---

# Language Constraints

Neko uses:

- immutable variables
- expression-oriented syntax
- block scoping
- no semicolons required (future phase)
- simplified pattern matching

Current lexer version may still tokenize semicolons for compatibility with examples.

---

# Lexer Responsibilities

The lexer must:

- validate every symbol belongs to the alphabet
- tokenize the source code
- classify lexemes
- generate lexical errors
- generate a symbol table
- export results into TXT files

---

# Output Files

Generate:

- output/tokens.txt
- output/symbols.txt
- output/errors.txt

---

# Code Style

- Write clean and heavily documented Rust code.
- Use descriptive names.
- Explain DFA transitions with comments.
- Avoid unnecessary abstractions.
- Prioritize readability over optimization.

---

# Important

This is an academic compiler project.

The implementation must prioritize:

- clarity
- explicit automata behavior
- educational value

over industrial optimization.

Avoid advanced macros or metaprogramming.

---

# Token Categories

Keywords:

- neko
- nyan
- fn
- if
- else
- match
- meow
- true
- false

Identifiers:
[a-zA-Z][a-zA-Z0-9_]\*

Integers:
[0-9]+

Strings:
"([a-zA-Z0-9_ ])\*"

Comments:
#.\*

Operators:

- - - / % = == != < > <= >= && || !

Delimiters:
( ) { } ,

---

# Example Input

fn neko {

    nyan score = 90

    nyan result = match score {
        100 => "Perfect"
        90  => "Excellent"
        _   => "Fail"
    }

    meow(result)

}
