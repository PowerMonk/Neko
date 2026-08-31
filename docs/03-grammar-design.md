# Neko — Diseño de la Gramática

> Documento **03** de la serie [`docs/`](./00-index.md).
> Complementa a [`02-grammar.md`](./02-grammar.md) justificando **por qué**
> la gramática tiene la forma que tiene y **cómo** se conecta con el
> compilador Rust (`neko-compiler`).

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

- Sin recursión izquierda (excepto en los operadores binarios, donde
  modelamos la izquierda-asociatividad con `*Tail` recursivos — ver §2.4).
- Cada alternativa de una producción se elige inspeccionando un único
  token (FIRST set unitario en la mayoría de los casos).

Justificación: el equipo está migrando un proyecto TypeScript mental a
Rust; un parser manual escrito en Rust es más legible que generar código
desde una tabla LALR(1).

### 2.2 Expresiones primero, statements después

En Neko **todo es una expresión** (excepto declaraciones e impresión).
Esa filosofía se respeta haciendo que:

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

### 2.4 Precedencia y asociatividad por niveles de no terminal — estilo left-recursive

Cada nivel de precedencia se modela con **dos** no terminales: el
"cabezal" (que consume el operando izquierdo) y el `*Tail` (que decide
si continuar con otro operando del mismo nivel). Esto produce árboles
de análisis sintáctico **anchos y planos** en lugar de altos y profundos:

```
Expr → OrExpr
OrExpr   → AndExpr OrTail
OrTail   → '||' AndExpr OrTail
         | ε
AndExpr  → NotExpr AndTail
AndTail  → '&&' NotExpr AndTail
         | ε
…
AddExpr  → MulExpr AddTail
AddTail  → '+' MulExpr AddTail
         | '-' MulExpr AddTail
         | ε
MulExpr  → Unary MulTail
MulTail  → '*' Unary MulTail
         | '/' Unary MulTail
         | '%' Unary MulTail
         | ε
Unary    → '-' Unary | Primary
Primary  → INT | STRING | 'true' | 'false' | ID | '(' Expr ')'
```

Para una expresión como `a + b * c`, el árbol se ve así (forma ancha):

```
Expr
└── OrExpr
    └── AndExpr
        └── NotExpr
            └── CmpExpr
                └── AddExpr
                    ├── MulExpr → Unary → Primary → ID 'a'
                    └── AddTail
                        ├── '+'
                        └── MulExpr
                            ├── Unary → Primary → ID 'b'
                            └── MulTail
                                ├── '*'
                                └── Unary → Primary → ID 'c'
                                └── ε  (cierra MulTail)
                        └── ε  (cierra AddTail)
```

Para una referencia simple como `score`, la cadena es mucho más corta
que la versión right-recursive:

```
Expr
└── OrExpr
    └── AndExpr
        └── NotExpr
            └── CmpExpr
                └── AddExpr
                    └── MulExpr
                        └── Unary
                            └── Primary
                                └── ID 'score'
```

(Las tres ramas `*Tail → ε` se marcan como nodos terminales `ε` en el
árbol Mermaid para hacer explícito el cierre de la cadena.)

### 2.5 Iteradores con nodo dedicado (`Stmts`, `Arms`)

Las repeticiones se modelan con un no-terminal explícito que se llama a
sí mismo y que termina con `ε`. Esto permite:

- Que el árbol de análisis sintáctico muestre **cada iteración** como un
  nodo hijo del iterador, en cadena.
- Que el iterador vacío (cero repeticiones) sea visible como una rama
  `ε`, no como una "ausencia" implícita.

```
Stmts → Stmt Stmts | ε
Arms  → Arm  Arms  | ε
```

### 2.6 Sin `;`, sin unidad `()`

Neko elimina los terminadores de instrucción. La gramática lo refleja:
no existe el terminal `;` y los statements se separan por **saltos de
línea** o por cierre de bloque `}`. La fase de parsing en Rust puede:

- Ignorar saltos de línea (como hace ya el lexer con espacios en blanco), o
- Tratarlos como `STATEMENT_SEPARATOR` en un nivel superior si se quiere
  reportar errores más ricos.

### 2.7 `meow` como statement

`meow(expr)` se usa como efecto (imprimir). Permitirlo como sub-expresión
abriría dos preguntas: ¿qué tipo retorna? ¿puede anidarse? Para mantener
la gramática simple — y alineada con la semántica de "void" — se modela
exclusivamente como `PrintStmt` a nivel de `Stmt`.

### 2.8 Patrones de `match`: cuatro formas, una sola estructura

Las ramas de `match` siguen el patrón `Pat => Expr`. `Pat` cubre solo
formas que el lexer ya reconoce sin ambigüedad:

| Forma     | Origen léxico               |
| --------- | --------------------------- |
| `_`       | `WILDCARD`                  |
| `INT`     | `INTEGER`                   |
| `STRING`  | `STRING_LITERAL`            |
| `ID`      | `IDENTIFIER` (binding)      |
| `true`/`false` | `TRUE`/`FALSE` keywords |

No se permiten patrones anidados ni `|` en esta primera iteración porque
el lexer tampoco los emite. Si en una iteración futura se decide
soportarlos, bastará con extender la producción `Pattern`.

### 2.9 `fn neko { ... }`: cuerpo único, sin parámetros

La especificación dice _"declaración de funciones sin parámetros"_, por
lo que la regla `FnDecl` se simplifica a:

```
FnDecl → 'fn' 'neko' Block
```

Y `Block` exige exactamente **una** expresión, lo que encaja con el
"retorno implícito" del lenguaje: el valor del bloque es el valor del
cuerpo.

### 2.10 Sin declaraciones dentro de expresiones

`nyan = ...` solo aparece como statement (`VarDecl`), no como sub-expresión
de un `if` o un `match`. Esto:

- Evita ambigüedad (`if x { nyan y = 1 } else { ... }` ya no requiere
  decidir si la asignación es parte del `then`).
- Refuerza la **inmutabilidad** y el **scope por bloques** mencionados en
  la documentación: una variable nace y muere en su statement.

### 2.11 Comentarios: no aparecen en la gramática

Los comentarios son filtrados por el lexer (`Comment` se descarta o se
emite pero el parser lo ignora). Mantenerlos fuera de la gramática evita
ruido visual y refleja la práctica usual.

---

## 3. Alternativas explícitas en los árboles de análisis sintáctico

El árbol Mermaid (`neko_parse_tree.mmd`) muestra, en cada nodo con más
de una producción posible, **solo la alternativa seleccionada** (los hijos
visibles son los del camino elegido). Las alternativas no seleccionadas
se marcan como un nodo terminal `ε` cuando aplican, o simplemente no se
dibujan cuando son irrelevantes para el camino.

Esto significa:

- Cada `Stmt` tiene **una** alternativa visible (`FnDecl` **o** `VarDecl`
  **o** `PrintStmt`), nunca las tres a la vez.
- Cada `Expr` con tres alternativas (`if`, `match`, `OrExpr`) muestra
  los hijos de la alternativa seleccionada, y el resto del árbol no se
  expande.
- Cada `*Tail` (`OrTail`, `AndTail`, `AddTail`, `MulTail`) muestra
  explícitamente su rama `ε` cuando no hay más operadores del mismo nivel.

---

## 4. FIRST / FOLLOW de referencia (resumen)

> Útil cuando se escriba el parser en Rust. Solo las uniones relevantes:

| No terminal | FIRST                                       |
| ----------- | ------------------------------------------- |
| Stmts       | `fn`, `nyan`, `meow`, `if`, `match`, `ID`, `INT`, `STRING`, `(`, `!`, `-`, `true`, `false`, `ε` |
| Stmt        | `fn`, `nyan`, `meow`, `if`, `match`, `ID`, `INT`, `STRING`, `(`, `!`, `-`, `true`, `false` |
| FnDecl      | `fn`                                        |
| VarDecl     | `nyan`                                      |
| PrintStmt   | `meow`                                      |
| Expr        | `if`, `match`, `ID`, `INT`, `STRING`, `(`, `!`, `-`, `true`, `false` |
| OrExpr      | idem Expr                                   |
| Pattern     | `_`, `INT`, `STRING`, `ID`, `true`, `false` |

> Las uniones entre los distintos niveles de expresión (`OrExpr`,
> `AndExpr`, …) comparten FIRST, por lo que en el parser la cascada de
> llamadas se hace de adentro hacia afuera: `parse_or_expr` llama a
> `parse_and_expr`, etc.

---

## 5. Integración futura con el compilador Rust

Pseudocódigo del parser (será implementado en `src/parser/`):

```rust
fn parse_program(&mut self) -> Result<Program, SyntaxError> {
    let stmts = self.parse_stmts()?;
    Ok(Program { stmts })
}

fn parse_stmts(&mut self) -> Result<Vec<Stmt>, SyntaxError> {
    let mut stmts = Vec::new();
    while !self.is_eof() && !self.is_block_close() {
        stmts.push(self.parse_stmt()?);
    }
    Ok(stmts) // ε implícito al salir del while
}

fn parse_stmt(&mut self) -> Result<Stmt, SyntaxError> {
    match self.peek().kind {
        TokenKind::KeywordFn   => self.parse_fn_decl(),
        TokenKind::KeywordNyan => self.parse_var_decl(),
        TokenKind::KeywordMeow => self.parse_print_stmt(),
        _                      => Ok(Stmt::Expr(self.parse_expr()?)),
    }
}

// … y así para cada no terminal.
```

El AST nodoso se almacenará en `src/parser/ast.rs` y consumirá la misma
lista de `Token` que ya produce `lexer::Lexer`. Ver
[`05-parser.md`](./05-parser.md) para la implementación concreta.

---

## 6. Lo que **no** cubre esta primera iteración

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
