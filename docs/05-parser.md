# Neko Parser — Documento de Implementación

> Documento **05** de la serie [`docs/`](./00-index.md).
> Describe cómo el parser de Neko está estructurado en `src/parser/` y
> cómo conecta con el lexer y con el AST.

---

## 1. Posición en el pipeline

```
   .neko source
        │
        ▼
   ┌──────────┐
   │  lexer   │  ──►  output/tokens.txt, output/symbols.txt, output/errors.txt
   └────┬─────┘
        │ Vec<Token>
        ▼
   ┌──────────┐
   │  parser  │  ──►  output/parse_tree.txt, output/parse_errors.txt
   └──────────┘
        │ SyntacticAnalysis { program, errors }
        ▼
   (semantic analysis + codegen en iteraciones futuras)
```

El parser **solo corre si la fase léxica reportó cero errores** — la
rúbrica del proyecto dice _"Tomar como entrada un código LIBRE DE ERRORES
LÉXICOS y verificar con base en su gramática"_. Si hay errores léxicos,
los archivos `parse_tree.txt` / `parse_errors.txt` se escriben vacíos
para mantener la forma del directorio predecible.

---

## 2. Estructura de `src/parser/`

```
src/parser/
├── mod.rs           ← declaración de submódulos + re-exports
├── ast.rs           ← tipos del AST (Stmt, Expr, Chain, Pattern, Arm, operadores)
├── error.rs         ← SyntaxError (linea, columna, mensaje)
├── token_stream.rs  ← cursor de solo-lectura sobre Vec<Token>
└── parser.rs        ← Parser: un método parse_xxx por no-terminal
```

Cada archivo tiene un comentario de cabecera que explica **por qué**
existe. Los métodos privados del parser siguen la convención
`parse_<no_terminal>` para que la correspondencia 1-a-1 con las
producciones de [`02-grammar.md`](./02-grammar.md) sea visible.

---

## 3. Estilo: descenso recursivo LL(1)

Una función `parse_xxx` por cada no-terminal de la gramática. Cada
elección entre alternativas se hace mirando un único token
(`peek().kind`). No hay tabla de parsing — el código ES la tabla.

Ejemplo (de `parser.rs`):

```rust
fn parse_expr(&mut self) -> Box<Expr> {
    match self.stream.peek_kind() {
        Some(TokenKind::KeywordIf)    => Box::new(self.parse_if_expr()),
        Some(TokenKind::KeywordMatch) => Box::new(self.parse_match_expr()),
        _                             => Box::new(self.parse_or_expr()),
    }
}
```

Esto corresponde directamente a:

```
Expr → 'if' Expr Block ElsePart
     | 'match' Expr '{' Arms '}'
     | OrExpr
```

---

## 4. AST vs árbol de análisis sintáctico

### Por qué el AST es más pequeño que el árbol de parseo

La gramática tiene **ocho** niveles de precedencia para expresiones:

```
Expr → OrExpr → AndExpr → NotExpr → CmpExpr → AddExpr → MulExpr → Unary → Primary
```

Para una variable simple como `score`, el **árbol de parseo** desciende
los ocho niveles hasta llegar a `Primary → ID "score"`. El **AST**,
en cambio, colapsa esa cadena en un único nodo `Expr::Or` (con `tail`
vacío) cuyo `left` es directamente el `Ident("score")`.

La forma exacta del AST está documentada en `src/parser/ast.rs`; en
resumen:

```
Expr::Or(Chain { left, ops: [], operands: [] })   ←  para `score`
Expr::Add(Chain { left: a, ops: [Plus], operands: [b] })   ←  para `a + b`
Expr::Add(Chain { left: a, ops: [Plus, Minus], operands: [b, c] })   ←  para `a + b - c`
```

### Regla de emparejamiento de los Vecs

Para cadenas left-recursivas como `AddExpr`:

```
AddExpr → MulExpr AddTail
AddTail → '+' MulExpr AddTail | '-' MulExpr AddTail | ε
```

El parser implementa esto como un **único loop**:

```rust
fn parse_add_expr(&mut self) -> Expr {
    let left = self.parse_mul_expr();
    let mut ops = Vec::new();
    let mut operands = Vec::new();
    loop {
        if consume(Plus)  { ops.push(Plus);  operands.push(parse_mul_expr()); }
        else if consume(Minus) { ops.push(Minus); operands.push(parse_mul_expr()); }
        else { break; }
    }
    Expr::Add(Chain { left: Box::new(left), ops, operands })
}
```

`ops[i]` se posiciona entre `left` (cuando i==0) o `operands[i-1]`
(cuando i>0) y `operands[i]`. La invariante es:

```
ops.len() == operands.len()
ops.len() < operands.len() + 1   (siempre hay al menos un operando: `left`)
```

### Por qué no usar `Vec<(Op, Operand)>`

Hubiera sido tentador definir `Chain { left, tail: Vec<(Op, Expr)> }`,
pero mezclar el operador y el operando en una tupla impide que el
matcher de Rust detecte errores de uso. Por ejemplo, un `Vec<(AddOp,
Expr)>` no se puede confundir con un `Vec<(MulOp, Expr)>`, pero el
matcher tampoco nos protege de pasar el tipo equivocado. Con dos Vecs
separadas la firma del tipo es explícita: `Chain<AddOpChain>` solo
acepta operadores de suma.

---

## 5. Manejo de errores: collect-don't-bail

El parser **nunca aborta** en el primer error. Cada `parse_xxx` que
detecta un problema:

1. Llama a `self.record_error(...)` para añadir el error a `self.errors`.
2. Intenta hacer el menor avance posible para que el siguiente intento
   tenga una posición razonable.
3. Continúa con el resto del análisis.

Esto produce listas de errores útiles (similar al lexer). Ejemplo con
`examples/broken.neko` (falta un `)` en el `meow`):

```
$ cargo run -- examples/broken.neko
Errors found — lexical: 0, syntactic: 1. See output/errors.txt and output/parse_errors.txt.
$ cat output/parse_errors.txt
9:1 -> Expected ')' after expression
```

---

## 6. Por qué `Block` es `Vec<Stmt>` y no `Expr`

La versión inicial de la gramática tenía:

```
Block → '{' Expr '}'
```

Pero el ejemplo canónico `examples/basic.neko` tiene un cuerpo de
función con **múltiples statements**:

```neko
fn neko {
    nyan banana = 90
    nyan holamundo = "Hola mundo desde Neko"
    nyan result = match score { ... }
    meow(result)
}
```

El primer intento del parser (con `Block = '{' Expr '}') consumía solo
la primera expresión (`nyan banana = 90`) y luego buscaba el `}`
inmediato, fallando en el resto del programa. La solución fue
**dividir** la producción `Block` en dos funciones:

| Función             | Contexto                                 | Devuelve     |
| ------------------- | ---------------------------------------- | ------------ |
| `parse_block`       | cuerpo de `fn neko { ... }`              | `Vec<Stmt>`  |
| `parse_block_expr`  | then/else de `if`, cuerpo de arm de `match` | `Box<Expr>` |

El AST de `Stmt::FnDecl` cambió de `{ body: Box<Expr> }` a
`{ body: Vec<Stmt> }`. La gramática en
[`02-grammar.md`](./02-grammar.md) ahora dice explícitamente que `Block`
contiene `Stmts` para reflejar esto.

---

## 7. Mensaje de terminal

`src/main.rs` resume ambas fases en una sola línea:

```
OK — 32 tokens, 4 symbols, parse tree built with 0 syntactic errors.
```

o en caso de errores:

```
Errors found — lexical: 0, syntactic: 1. See output/errors.txt and output/parse_errors.txt.
```

Los archivos exactos a inspeccionar se mencionan por nombre para que el
usuario no tenga que adivinar.

---

## 8. Archivos de salida

| Archivo                  | Contenido                                                  |
| ------------------------ | ---------------------------------------------------------- |
| `output/tokens.txt`      | Un token por línea: `KIND -> lexema`                       |
| `output/symbols.txt`     | Tabla de identificadores (uno por línea, sin duplicados)   |
| `output/errors.txt`      | Errores léxicos: `linea:col -> mensaje`                    |
| `output/parse_tree.txt`  | AST en forma de árbol indentado (2 espacios por nivel)     |
| `output/parse_errors.txt`| Errores sintácticos: `linea:col -> mensaje`               |

---

## 9. Lo que falta para iteraciones futuras

- Análisis semántico (tabla de tipos, verificación de uso de variables).
- Generación de código ensamblador.
- Más tests unitarios dentro de `src/parser/` (hoy solo hay cobertura
  end-to-end vía `cargo run -- examples/basic.neko`).

Ver [`03-grammar-design.md`](./03-grammar-design.md) §6 para la lista
completa de extensiones planeadas (match con `|`, parámetros en fn,
etc.).

---

## 10. Programas de ejemplo

El repositorio incluye tres programas de ejemplo en `examples/`:

| Archivo                       | Qué demuestra                                                                                |
| ----------------------------- | -------------------------------------------------------------------------------------------- |
| `examples/basic.neko`         | El programa canónico: FnDecl con VarDecl, match de 3 brazos, y meow. Usado por el lexer.     |
| `examples/operators.neko`     | Cadenas de operadores (`a + b + c`, `a * b * c`). Útil para verificar el AST aplanado.      |
| `examples/broken.neko`        | Programa con un `)` faltante en un `meow`. Para probar la recuperación de errores.           |

Para ejecutar cualquiera:

```
$ cargo run -- examples/operators.neko
OK — 40 tokens, 5 symbols, parse tree built with 0 syntactic errors.

$ cargo run -- examples/broken.neko
Errors found — lexical: 0, syntactic: 1. See output/errors.txt and output/parse_errors.txt.
```
