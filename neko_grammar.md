# Neko — Gramática

> Gramática formal del lenguaje **Neko** (notación BNF extendida).
> Diseñada para ser **no ambigua** y **factible mediante análisis sintáctico
> descendente recursivo (LL(1))**, lo que permitirá integrarla en el módulo
> `parser/` del compilador Rust (`neko-compiler`) en una iteración futura.

---

## 1. Notación

| Símbolo            | Significado                                          |
|--------------------|------------------------------------------------------|
| `→`                | "produce" / "se deriva en"                           |
| `\|`               | Alternativas                                        |
| `ε`                | Producción vacía                                     |
| `'x'`              | Terminal literal (palabra clave, operador, símbolo)  |
| `MAYÚSCULAS`       | No terminal                                          |
| `cursiva`          | Metavariable / patrón                                |

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
Program        → Stmt*

Stmt           → FnDecl
               | VarDecl
               | PrintStmt

FnDecl         → 'fn' 'neko' Block

VarDecl        → 'nyan' ID '=' Expr

PrintStmt      → 'meow' '(' Expr ')'

Block          → '{' Expr '}'

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

OrExpr         → AndExpr ( '||' AndExpr )*

AndExpr        → NotExpr ( '&&' NotExpr )*

NotExpr        → '!' NotExpr
               | CmpExpr

CmpExpr        → AddExpr CmpTail

CmpTail        → CmpOp AddExpr
               | ε

CmpOp          → '==' | '!=' | '<' | '>' | '<=' | '>='

AddExpr        → MulExpr ( ('+' | '-') MulExpr )*

MulExpr        → Unary ( ('*' | '/' | '%') Unary )*

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
|-------|-------------------------------------------|---------------|
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

## 5. Ejemplo de programa

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

## 6. Derivación (árbol de análisis sintáctico)

La derivación completa del programa anterior — siguiendo la gramática —
produce el siguiente árbol de análisis sintáctico:

```
Program
├── FnDecl
│   ├── 'fn'
│   ├── 'neko'
│   └── Block
│       ├── '{'
│       └── Expr
│           └── OrExpr
│               └── AndExpr
│                   └── NotExpr
│                       └── CmpExpr
│                           └── AddExpr
│                               └── MulExpr
│                                   └── Unary
│                                       └── Primary
│                                           └── ID  ("score")
├── VarDecl
│   ├── 'nyan'
│   ├── ID  ("result")
│   ├── '='
│   └── Expr
│       └── 'match'
│           ├── Expr  →  ID "score"
│           ├── '{'
│           ├── Arms
│           │   ├── Arm
│           │   │   ├── Pattern  →  INT "100"
│           │   │   ├── '=>'
│           │   │   └── Expr
│           │   │       └── … → Primary STRING "Perfect"
│           │   ├── Arm
│           │   │   ├── Pattern  →  INT "90"
│           │   │   ├── '=>'
│           │   │   └── Expr  →  Primary STRING "Excellent"
│           │   └── Arm
│           │       ├── Pattern  →  '_'
│           │       ├── '=>'
│           │       └── Expr  →  Primary STRING "Fail"
│           └── '}'
└── PrintStmt
    ├── 'meow'
    ├── '('
    ├── Expr  →  ID "result"
    └── ')'
```

> El árbol Mermaid completo se encuentra en [`neko_parse_tree.mmd`](./neko_parse_tree.mmd).

---

## 7. Verificación rápida

Para cada regla se cumplen las dos condiciones que exige el analizador
sintáctico del curso:

1. **Cobertura**: cada construcción usa **todos** los componentes léxicos que
   requiere (`fn neko { ... }` usa exactamente esos 4 tokens; `match` usa
   `match`, `{`, `}`, etc.).
2. **Orden**: los terminales aparecen en el orden prescrito por cada
   producción, de izquierda a derecha.

Si el analizador léxico produce una cadena de tokens y esta gramática genera
un árbol de análisis sintáctico, entonces el programa es **sintácticamente
válido** para Neko.
