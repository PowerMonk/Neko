# Neko — Diseño de la Gramática

> Documento complementario a [`neko_grammar.md`](./neko_grammar.md).
> Aquí se justifica cada decisión de diseño: **por qué** la gramática tiene
> la forma que tiene y **cómo** se conectará con el resto del compilador
> Rust (`neko-compiler`).

---

## 1. Punto de partida: el lexer ya emite tokens

La fase léxica (ver `src/lexer/`) produce una secuencia de tokens. El
analizador sintáctico **asume** que esa secuencia está libre de errores
léxicos (tal como lo exige la rúbrica del curso) y se enfoca en dos
preguntas:

1. ¿La cadena de tokens **cubre todos los componentes** que requiere cada
   estructura?
2. ¿Aparecen **en el orden correcto**?

La gramática que define Neko debe responder ambas preguntas de forma
**mecánica y determinista** — sin reglas ambiguas que conduzcan a más de
un árbol de análisis posible.

---

## 2. Decisiones de diseño

### 2.1 Estilo LL(1) / descendente recursivo

Se eligió una gramática susceptible de analizarse con **descenso recursivo
predictivo** (un parser por no terminal, uno o dos tokens de lookahead):

- Sin recursión izquierda.
- Sin factorización pendiente significativa.
- Cada alternativa de una producción se puede elegir inspeccionando un
  único token (FIRST set unitario en la mayoría de los casos).

Justificación: el equipo está migrando un proyecto TypeScript mental a
Rust; un parser manual escrito en Rust es más legible que generar código
desde una tabla LALR(1).

### 2.2 Expresiones primero, statements después

En Neko **todo es una expresión** (excepto declaraciones e impresión). Esa
filosofía se respeta haciendo que:

- `Stmt` solo cubre formas "efecto": `FnDecl`, `VarDecl`, `PrintStmt`.
- `Expr` cubre **todo lo demás**, incluyendo `if`, `match` y cualquier
  combinación de operadores.

Esto refleja el lema de Neko: _"menos sintaxis, mayor expresividad"_.

### 2.3 `if` y `match` como expresiones, no como statements

`if` y `match` **producen un valor** (ver restricciones semánticas en
`Neko_intro.md`). Por eso viven dentro de `Expr`, no en `Stmt`:

```
nyan grade = if score > 70 { "Pass" } else { "Fail" }
```

Esto evita una gramática con un `IfStmt` separado que luego tendría que
duplicar casi todas las reglas para `IfExpr`.

### 2.4 Precedencia por niveles de no terminal

Cada nivel de precedencia se modela como un no terminal distinto
(`OrExpr` → `AndExpr` → `NotExpr` → `CmpExpr` → `AddExpr` → `MulExpr` →
`Unary` → `Primary`). Es la técnica clásica para evitar ambigüedad en
expresiones con múltiples operadores sin usar `%prec` ni directivas
específicas de un generador (Yacc/Bison).

### 2.5 Sin `;`, sin unidad `()`

Neko elimina los terminadores de instrucción. La gramática lo refleja: no
existe el terminal `;` y los statements se separan por **saltos de línea**
o por cierre de bloque `}`. La fase de parsing en Rust puede:

- Ignorar saltos de línea (como hace ya el lexer con espacios en blanco), o
- Tratarlos como `STATEMENT_SEPARATOR` en un nivel superior si se quiere
  reportar errores más ricos.

### 2.6 `meow` como statement

`meow(expr)` se usa como efecto (imprimir). Permitirlo como sub-expresión
abriría dos preguntas: ¿qué tipo retorna? ¿puede anidarse? Para mantener
la gramática simple — y alineada con la semántica de "void" — se modela
exclusivamente como `PrintStmt` a nivel de `Stmt`.

### 2.7 Patrones de `match`: cuatro formas, una sola estructura

Las ramas de `match` siguen el patrón `Pat => Expr`. `Pat` cubre solo
formas que el lexer ya reconoce sin ambigüedad:

| Forma     | Origen léxico               |
|-----------|-----------------------------|
| `_`       | `WILDCARD`                  |
| `INT`     | `INTEGER`                   |
| `STRING`  | `STRING_LITERAL`            |
| `ID`      | `IDENTIFIER` (binding)      |
| `true`/`false` | `TRUE`/`FALSE` keywords |

No se permiten patrones anidados ni `|` en esta primera iteración porque
el lexer tampoco los emite. Si en una iteración futura se decide
soportarlos, bastará con extender la producción `Pattern`.

### 2.8 `fn neko { ... }`: cuerpo único, sin parámetros

La especificación dice _"declaración de funciones sin parámetros"_, por lo
que la regla `FnDecl` se simplifica a:

```
FnDecl → 'fn' 'neko' Block
```

Y `Block` exige exactamente **una** expresión, lo que encaja con el
"retorno implícito" del lenguaje: el valor del bloque es el valor del
cuerpo.

### 2.9 Sin declaraciones dentro de expresiones

`nyan = ...` solo aparece como statement (`VarDecl`), no como sub-expresión
de un `if` o un `match`. Esto:

- Evita ambigüedad (`if x { nyan y = 1 } else { ... }` ya no requiere
  decidir si la asignación es parte del `then`).
- Refuerza la **inmutabilidad** y el **scope por bloques** mencionados en
  la documentación: una variable nace y muere en su statement.

### 2.10 Comentarios: no aparecen en la gramática

Los comentarios son filtrados por el lexer (`Comment` se descarta o se
emite pero el parser lo ignora). Mantenerlos fuera de la gramática evita
ruido visual y refleja la práctica usual.

---

## 3. FIRST / FOLLOW de referencia (resumen)

> Útil cuando se escriba el parser en Rust. Solo las uniones relevantes:

| No terminal | FIRST                                  |
|-------------|----------------------------------------|
| Stmt        | `fn`, `nyan`, `meow`, `if`, `match`, `ID`, `INT`, `STRING`, `(`, `!`, `-`, `true`, `false` |
| FnDecl      | `fn`                                   |
| VarDecl     | `nyan`                                 |
| PrintStmt   | `meow`                                 |
| Expr        | `if`, `match`, `ID`, `INT`, `STRING`, `(`, `!`, `-`, `true`, `false` |
| OrExpr      | idem Expr                              |
| Pattern     | `_`, `INT`, `STRING`, `ID`, `true`, `false` |

> Las uniones entre los distintos niveles de expresión (`OrExpr`,
> `AndExpr`, …) comparten FIRST, por lo que en el parser la cascada de
> llamadas se hace de adentro hacia afuera: `parse_or_expr` llama a
> `parse_and_expr`, etc.

---

## 4. Integración futura con el compilador Rust

Pseudocódigo del parser (será implementado en `src/parser/`):

```rust
fn parse_program(&mut self) -> Result<Program, SyntaxError> {
    let mut stmts = Vec::new();
    while !self.is_eof() {
        stmts.push(self.parse_stmt()?);
    }
    Ok(Program { stmts })
}

fn parse_stmt(&mut self) -> Result<Stmt, SyntaxError> {
    match self.peek().kind {
        TokenKind::KeywordFn    => self.parse_fn_decl(),
        TokenKind::KeywordNyan  => self.parse_var_decl(),
        TokenKind::KeywordMeow  => self.parse_print_stmt(),
        _                       => Ok(Stmt::Expr(self.parse_expr()?)),
    }
}

// … y así para cada no terminal.
```

El AST nodoso se almacenará en `src/parser/ast.rs` y consumirá la misma
lista de `Token` que ya produce `lexer::Lexer`.

---

## 5. Lo que **no** cubre esta primera iteración

A propósito, para no inflar la gramática antes de validar que el flujo
léxico → sintáctico funciona end-to-end:

- `match` con varios valores por brazo (`1 | 2 => ...`).
- Patrones de tupla o rango.
- Funciones con parámetros (`fn neko(x, y) { ... }`).
- Shadowing explícito o reasignación.
- Arrays, maps, módulos.

Cualquiera de estos se añadirá en una iteración posterior, **solo**
después de que esta gramática base esté integrada y produciendo árboles
de análisis sintáctico válidos para programas reales.
