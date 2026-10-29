# Neko

> Un lenguaje de programación compilado, minimalista y orientado a
> expresiones — y su compilador, escrito en Rust.

**Neko** es un proyecto académico de la materia *Lenguajes y Autómatas 1*.
El repositorio contiene el compilador completo, desde el análisis léxico
hasta la generación de código ensamblador para Linux x86-64.

---

## El lenguaje en 30 segundos

```neko
fn neko {
    nyan score = 90

    nyan result = match score {
        100 => "Perfect"
        90  => "Excellent"
        _   => "Fail"
    }

    meow(result)
}
```

- `fn neko { … }` — el punto de entrada del programa.
- `nyan nombre = valor` — declara una variable (inmutable, con tipo inferido).
- `match expr { patrón => valor, … _ => valor }` — coincidencia de patrones.
- `if expr { … } else { … }` — condicional que produce un valor.
- `meow(expr)` — imprime en la pantalla (sin salto de línea final).
- `#` — comentario de una sola línea.

Tipos: `int`, `string`, `bool`. Sin punto y coma: las instrucciones
terminan en salto de línea o en `}`.

---

## Compilar y ejecutar

### Lo más rápido (Windows con WSL)

```powershell
cargo run -- examples/basic.neko
```

Esto hace **todo**: analiza, genera `output/program.asm`, lo ensambla con
`nasm`, lo enlaza con `ld` y ejecuta el binario, mostrando su salida en la
terminal.

```
OK — 44 tokens, 4 symbols, parse tree built with 0 syntactic errors,
     semantic analysis: 0 errors, intermediate code: 21 quadruples, 21 triples,
     assembly: output/program.asm.

--- ejecutando el programa compilado ---
ExcellentHola mundo desde Neko90
```

### En Linux o una VM de Linux

```bash
cargo run -- examples/basic.neko
```

El compilador detecta que no hay WSL y muestra las instrucciones manuales
en su lugar.

### Solo generar el `.asm`, sin ejecutarlo

```powershell
cargo run -- examples/basic.neko --no-run
```

---

## Cómo ejecutar el `.asm` a mano

El archivo generado (`output/program.asm`) es **autocontenido**: incluye
sus propias rutinas de runtime. Se puede ensamblar y ejecutar en cualquier
Linux sin nada más instalado.

### En Linux (o WSL)

```bash
cd output
nasm -f elf64 program.asm -o program.o
ld program.o -o program
./program
```

### Desde Windows usando WSL

```powershell
wsl -- bash -c "cd /mnt/c/ruta/a/Neko/output && nasm -f elf64 program.asm -o program.o && ld program.o -o program && ./program"
```

O, si prefieres entrar a WSL primero:

```powershell
wsl
cd /mnt/c/ruta/a/Neko/output
nasm -f elf64 program.asm -o program.o
ld program.o -o program
./program
```

### Requisitos

| Herramienta | Para qué                    | ¿Viene con...?            |
|-------------|----------------------------|---------------------------|
| `nasm`      | ensamblar el `.asm`        | se instala aparte        |
| `ld`        | enlazar el `.o`             | `binutils`, viene con gcc |
| Linux x86-64| ejecutar el binario         | WSL, VM o máquina propia  |

> **¿Ya tienes WSL?** En este proyecto se validó con Fedora Linux 42
> dentro de WSL: `nasm 2.16.03`, `ld`, y `x86_64`. Todo funciona.

---

## El pipeline del compilador

```
   fuente .neko
        │
        ├── 1. Análisis léxico     (src/lexer/)     → tokens, tabla de símbolos, errores léxicos
        ├── 2. Análisis sintáctico (src/parser/)    → árbol sintáctico, errores sintácticos
        ├── 3. Análisis semántico  (src/semantic/)  → tabla de símbolos con tipos, errores semánticos
        ├── 4. Código intermedio   (src/icg/)       → cuadruplos y triplos (3AC)
        └── 5. Código ensamblador  (src/codegen/)   → output/program.asm
```

Cada fase sólo corre si todas las anteriores salieron limpias. Si
una fase encuentra errores, las siguientes se omiten y se escribe un
marcador en su archivo de salida.

### Los archivos que produce

| Archivo                    | Contenido                                            |
|----------------------------|------------------------------------------------------|
| `output/tokens.txt`        | Un token por línea: `KIND -> lexema`                 |
| `output/symbols.txt`       | Identificadores declarados (fase léxica)             |
| `output/errors.txt`        | Errores léxicos: `linea:col -> mensaje`              |
| `output/parse_tree.txt`    | El AST en forma de árbol indentado                   |
| `output/parse_errors.txt`  | Errores sintácticos                                   |
| `output/semantic_errors.txt`| Errores semánticos                                  |
| `output/quadruples.txt`    | Cuádruplos: `( op , arg1 , arg2 , resultado )`        |
| `output/triples.txt`       | Triplos: `( op , arg1 , arg2 )` con índice           |
| `output/symbol_table.txt`  | Tabla de símbolos con direcciones                    |
| `output/program.asm`       | **El programa en ensamblador NASM**                   |

---

## Ejemplos

```
examples/
├── basic.neko              ← el programa canónico
├── operators.neko          ← cadenas de operadores aritméticos
├── broken.neko             ← error sintáctico (falta un `)`)
├── lex-error.neko          ← error detectable por el lexer
├── syn-error.neko          ← error detectable por el parser
├── sem-error.neko          ← error detectable por el semántico
├── sem-tests/              ← 8 programas para el análisis semántico
│   ├── sem-test-tipos-ok.neko           sem-test-tipos-error.neko
│   ├── sem-test-compatibilidad-ok.neko  sem-test-compatibilidad-error.neko
│   ├── sem-test-unicidad-ok.neko        sem-test-unicidad-error.neko
│   └── sem-test-funcion-ok.neko         sem-test-funcion-error.neko
└── icg-tests/              ← programas para la generación de código
    ├── icg-ok.neko         ← aritmética, if/else y match de 3 brazos
    └── icg-if-true.neko    ← la rama verdadera del if/else
```

Para la demostración en video, estos cuatro ejercitan una fase distinta
cada uno:

```powershell
cargo run -- examples/lex-error.neko
cargo run -- examples/syn-error.neko
cargo run -- examples/sem-error.neko
cargo run -- examples/icg-tests/icg-ok.neko
```

---

## Tests

```bash
cargo test
```

78 pruebas unitarias, todas sin I/O:

| Módulo            | Pruebas | Qué cubren                                       |
|-------------------|---------|--------------------------------------------------|
| `src/parser/`     | 8       | Gramática, recuperación de errores                |
| `src/semantic/`   | 16      | Las 4 comprobaciones estáticas + auxiliares       |
| `src/icg/`        | 20      | Cuádruplos, triplos, iteradores                   |
| `src/codegen/`    | 34      | Traducción a ensamblador, tabla de símbolos, WSL  |

---

## Documentación

La serie completa de documentos está en [`docs/`](./docs/), numerada para
leerse en orden:

| # | Documento | Tema |
|---|-----------|------|
| 00 | [`00-index.md`](./docs/00-index.md) | Índice de la serie |
| 01 | [`01-lexer.md`](./docs/01-lexer.md) | Análisis léxico |
| 02 | [`02-grammar.md`](./docs/02-grammar.md) | Gramática formal de Neko |
| 03 | [`03-grammar-design.md`](./docs/03-grammar-design.md) | Decisiones de diseño de la gramática |
| 04 | [`04-parse-tree-example.mmd`](./docs/04-parse-tree-example.mmd) | Árbol sintáctico (Mermaid) |
| 05 | [`05-parser.md`](./docs/05-parser.md) | Análisis sintáctico en Rust |
| 06 | [`06-semantic-conventions.md`](./docs/06-semantic-conventions.md) | Convenciones semánticas |
| 07 | [`07-semantic-implementation.md`](./docs/07-semantic-implementation.md) | Análisis semántico en Rust |
| 08 | [`08-icg-implementation.md`](./docs/08-icg-implementation.md) | Código intermedio |
| 09 | [`09-codegen-implementation.md`](./docs/09-codegen-implementation.md) | Generación de código ensamblador |

---

## Estructura del proyecto

```
Neko/
├── Cargo.toml
├── docs/                  ← toda la documentación
├── examples/              ← programas de prueba
├── output/                ← artefactos generados (se regeneran al compilar)
└── src/
    ├── main.rs            ← pipeline principal
    ├── lexer/             ← fase 1: tokenización
    │   ├── alphabet.rs    ← clasificadores de caracteres
    │   ├── automata.rs    ← reconocedores DFA
    │   ├── lexer.rs       ← orquestador del escaneo
    │   ├── symbol_table.rs← tabla de identificadores
    │   └── token.rs       ← TokenKind, Token, LexicalError
    ├── parser/            ← fase 2: análisis sintáctico
    │   ├── ast.rs         ← tipos del AST
    │   ├── error.rs       ← SyntaxError
    │   ├── parser.rs      ← descenso recursivo
    │   └── token_stream.rs← cursor sobre los tokens
    ├── semantic/          ← fase 3: análisis semántico
    │   ├── analyzer.rs    ← visitor + tabla de ámbito
    │   ├── error.rs       ← SemanticError
    │   ├── scope.rs       ← Scope y Symbol
    │   └── types.rs       ← Type { Int, String, Bool, Fn }
    ├── icg/               ← fase 4: código intermedio
    │   ├── emitter.rs     ← contadores de T_n y L_n
    │   ├── generator.rs   ← visitor que produce los cuadruplos
    │   ├── quadruple.rs   ← Quadruple
    │   └── triple.rs      ← Triple
    └── codegen/           ← fase 5: código ensamblador
        ├── assembler.rs   ← tradúce los cuadruplos a NASM
        ├── runner.rs      ← ensambla, enlaza y ejecuta vía WSL
        ├── runtime.rs     ← rutinas del runtime
        └── symbol_table.rs← tabla de símbolos con direcciones
```

**Sin dependencias externas.** Todo el proyecto usa únicamente la
biblioteca estándar de Rust.

---

## Autoría

Proyecto de la materia *Lenguajes y Autómatas 1*.
