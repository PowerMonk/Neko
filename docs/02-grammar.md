# Neko — Gramática

> Gramática formal del lenguaje **Neko** (notación BNF extendida).
> Documento **02** de la serie [`docs/`](./00-index.md).
> Diseñada para ser **no ambigua** y para producir **árboles de análisis
> sintáctico planos y legibles**, siguiendo la convención clásica de
> operadores **left-recursive** (Aho/Sethi/Ullman).
> Esta gramática se integrará en el módulo `parser/` del compilador Rust
> (`neko-compiler`) en una iteración futura.

---

## 1. Notación

| Símbolo      | Significado                                         |
| ------------ | --------------------------------------------------- |
| `→`          | "produce" / "se deriva en"                          |
| `\|`         | Alternativas                                        |
| `ε`          | Producción vacía                                    |
| `'x'`        | Terminal literal (palabra clave, operador, símbolo) |
| `MAYÚSCULAS` | No terminal                                         |

---

## 2. Tokens (provenientes del lexer)

```
'neko' 'nyan' 'fn' 'if' 'else' 'match' 'meow' 'true' 'false'
INT  STRING  ID  '_'
'+'  '-'  '*'  '/'  '%'
'='  '=='  '!='  '<'  '>'  '<='  '>='  '!'  '&&'  '||'  '=>'
'('  ')'  '{'  '}'  ','
```

> Nota: en Neko **no existe** el terminador `;`. Las instrucciones terminan
> por salto de línea o por cierre de bloque `}`.

---

## 3. Gramática

```
Program        → Stmts

Stmts          → Stmt Stmts
               | ε

Stmt           → FnDecl
               | VarDecl
               | PrintStmt

FnDecl         → 'fn' 'neko' Block

VarDecl        → 'nyan' ID '=' Expr

PrintStmt      → 'meow' '(' Expr ')'

> **Nota**: `Block` contiene una secuencia de statements (`Stmts`), no una
> sola expresión. Esto permite que `fn neko { ... }` tenga un cuerpo con
> múltiples declaraciones, como en `examples/basic.neko`. La idea de
> "retorno implícito" se mantiene porque el último statement del bloque
> sigue siendo una expresión-evaluación.

Block          → '{' Stmts '}'

Expr           → 'if' Expr Block ElsePart
               | 'match' Expr '{' Arms '}'
               | OrExpr

ElsePart       → 'else' Block
               | ε

Arms           → Arm Arms
               | ε

Arm            → Pattern '=>' Expr

Pattern        → '_'
               | INT
               | STRING
               | ID
               | 'true'
               | 'false'

OrExpr         → AndExpr OrTail

OrTail         → '||' AndExpr OrTail
               | ε

AndExpr        → NotExpr AndTail

AndTail        → '&&' NotExpr AndTail
               | ε

NotExpr        → '!' NotExpr
               | CmpExpr

CmpExpr        → AddExpr CmpTail

CmpTail        → CmpOp AddExpr
               | ε

CmpOp          → '==' | '!=' | '<' | '>' | '<=' | '>='

AddExpr        → MulExpr AddTail

AddTail        → '+' MulExpr AddTail
               | '-' MulExpr AddTail
               | ε

MulExpr        → Unary MulTail

MulTail        → '*' Unary MulTail
               | '/' Unary MulTail
               | '%' Unary MulTail
               | ε

Unary          → '-' Unary
               | Primary

Primary        → INT
               | STRING
               | 'true'
               | 'false'
               | ID
               | '(' Expr ')'
```

---

## 4. Precedencia y asociatividad

| Nivel | Operador(es)                              | Asociatividad |
| ----- | ----------------------------------------- | ------------- |
| 0     | `if` / `match` (expresión)                | —             |
| 1     | `\|\|`                                    | Izquierda     |
| 2     | `&&`                                      | Izquierda     |
| 3     | `!`                                       | Prefijo       |
| 4     | `==` `!=` `<` `>` `<=` `>=`               | Izquierda     |
| 5     | `+` `-`                                   | Izquierda     |
| 6     | `*` `/` `%`                               | Izquierda     |
| 7     | `-` unario                                | Prefijo       |
| 8     | Primarios (`INT`, `ID`, `(...)`, etc.)    | —             |

> `if` y `match` no son operadores binarios: ocupan su lugar como formas
> expresión completas, evitando ambigüedad con sus operandos.

---

## 5. Iteradores explícitos

Para que los árboles de análisis sintáctico reflejen **explícitamente**
la posibilidad de repetir elementos, las listas se modelan con un
no-terminal dedicado que se llama a sí mismo:

| Lista        | Producción                                       |
| ------------ | ------------------------------------------------ |
| `Stmts`      | `Stmts → Stmt Stmts \| ε`                        |
| `Arms`       | `Arms → Arm Arms \| ε`                           |
| `OrTail`     | `OrTail → '\|\|' AndExpr OrTail \| ε`            |
| `AndTail`    | `AndTail → '&&' NotExpr AndTail \| ε`            |
| `AddTail`    | `AddTail → '+' MulExpr AddTail \| '−' MulExpr AddTail \| ε` |
| `MulTail`    | `MulTail → '*' Unary MulTail \| '/' Unary MulTail \| '%' Unary MulTail \| ε` |

Cada `*Tail` representa el "siguiente operador" en una cadena del mismo
nivel de precedencia. La rama `ε` siempre existe y es la que cierra la
cadena cuando ya no hay más operadores del mismo nivel.

---

## 6. Ejemplo de programa

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

---

## 7. Derivación (árbol de análisis sintáctico)

La derivación completa del programa anterior — siguiendo la gramática —
produce el siguiente árbol de análisis sintáctico:

```
Program
└── Stmts
    ├── Stmt  (alt: FnDecl)
    │   ├── FnDecl
    │   │   ├── 'fn'
    │   │   ├── 'neko'
    │   │   └── Block
    │   │       ├── '{'
    │   │       └── Expr  (alt: OrExpr)
    │   │           └── OrExpr
    │   │               └── AndExpr
    │   │                   └── NotExpr
    │   │                       └── CmpExpr
    │   │                           └── AddExpr
    │   │                               └── MulExpr
    │   │                                   └── Unary
    │   │                                       └── Primary
    │   │                                           └── ID 'score'
    │   │       └── '}'
    │   ├── Stmt  (alt: VarDecl)
    │   │   ├── VarDecl
    │   │   │   ├── 'nyan'
    │   │   │   ├── ID 'score'
    │   │   │   ├── '='
    │   │   │   └── Expr  (alt: OrExpr)
    │   │   │       └── OrExpr → AndExpr → NotExpr → CmpExpr
    │   │   │           └── AddExpr
    │   │   │               ├── MulExpr
    │   │   │               │   └── Unary
    │   │   │               │       └── Primary
    │   │   │               │           └── INT '90'
    │   │   │               └── AddTail → ε
    │   │   │           └── CmpTail → ε
    │   │   │           └── NotTail → ε
    │   │   │           └── OrTail → ε
    │   │   │           └── Expr (alts 'if' y 'match' → ε)
    │   │   └── Stmt  (alt: VarDecl)
    │   │       ├── VarDecl
    │   │       │   ├── 'nyan'
    │   │       │   ├── ID 'result'
    │   │       │   ├── '='
    │   │       │   └── Expr  (alt: 'match')
    │   │       │       ├── 'match'
    │   │       │       ├── Expr  → ID 'score'
    │   │       │       ├── '{'
    │   │       │       ├── Arms
    │   │       │       │   ├── Arm
    │   │       │       │   │   ├── Pattern → INT '100'
    │   │       │       │   │   ├── '=>'
    │   │       │       │   │   └── Expr → Primary → STRING 'Perfect'
    │   │       │       │   ├── Arm
    │   │       │       │   │   ├── Pattern → INT '90'
    │   │       │       │   │   ├── '=>'
    │   │       │       │   │   └── Expr → Primary → STRING 'Excellent'
    │   │       │       │   └── Arm
    │   │       │       │       ├── Pattern → '_'
    │   │       │       │       ├── '=>'
    │   │       │       │       └── Expr → Primary → STRING 'Fail'
    │   │       │       │   └── Arms → ε
    │   │       │       └── '}'
    │   │       └── Stmt  (alt: PrintStmt)
    │   │           ├── PrintStmt
    │   │           │   ├── 'meow'
    │   │           │   ├── '('
    │   │           │   ├── Expr → ID 'result'
    │   │           │   └── ')'
    │   │           └── Stmts → ε
```

> El árbol Mermaid completo se encuentra en [`neko_parse_tree.mmd`](./neko_parse_tree.mmd).

---

## 8. Verificación rápida

Para cada regla se cumplen las dos condiciones que exige el analizador
sintáctico del curso:

1. **Cobertura**: cada construcción usa **todos** los componentes léxicos
   que requiere (`fn neko { ... }` usa exactamente esos 4 tokens;
   `match` usa `match`, `{`, `}`, etc.).
2. **Orden**: los terminales aparecen en el orden prescrito por cada
   producción, de izquierda a derecha.

Si el analizador léxico produce una cadena de tokens y esta gramática
genera un árbol de análisis sintáctico, entonces el programa es
**sintácticamente válido** para Neko.
