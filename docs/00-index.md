# Neko — Documentación

> Serie de documentos que describen el lenguaje **Neko** y su compilador
> Rust (`neko-compiler`). Leer en orden.

## Índice

| # | Documento                                   | Tema                                                                 |
| - | ------------------------------------------- | -------------------------------------------------------------------- |
| 01 | [`01-lexer.md`](./01-lexer.md)             | Análisis léxico: tokenización, autómatas, tabla de símbolos          |
| 02 | [`02-grammar.md`](./02-grammar.md)         | Gramática formal (BNF) de Neko                                       |
| 03 | [`03-grammar-design.md`](./03-grammar-design.md) | Decisiones de diseño de la gramática (precedencia, iteradores, etc.) |
| 04 | [`04-parse-tree-example.mmd`](./04-parse-tree-example.mmd) | Árbol de análisis sintáctico (Mermaid) del programa `basic.neko` |
| 05 | [`05-parser.md`](./05-parser.md)           | Análisis sintáctico: estructura del módulo `parser/` en Rust         |
| 06 | [`06-semantic-conventions.md`](./06-semantic-conventions.md) | Convenciones semánticas: alcance, tipos, unicidad, uso, exhaustividad |
| 07 | [`07-semantic-implementation.md`](./07-semantic-implementation.md) | Implementación del análisis semántico en Rust (`src/semantic/`) |
| 08 | [`08-icg-implementation.md`](./08-icg-implementation.md) | Generación de código intermedio: cuádruplos y triplos (`src/icg/`) |
| 09 | [`09-codegen-implementation.md`](./09-codegen-implementation.md) | Generación de código ensamblador x86-64 (`src/codegen/`) |

## Convención de nombres

Cada archivo lleva un prefijo numérico de dos dígitos para forzar el
orden de lectura en listados de directorio y en hipervínculos. El
número refleja el orden lógico del flujo del compilador:

```
01 (lexer)   →  02 (grammar) →  03 (design)  →  04 (parse tree) →  05 (parser)
   entrada        especificación    justificaciones     ejemplo visual    implementación
                                    ↓
06 (semantic conventions) →  07 (semantic impl.) →  08 (ICG) →  09 (codegen)
        contrato                implementación      intermedio      ensamblador
```

Documentos que se añadan después deben seguir la numeración: `10-...`,
`11-...`, etc.

## Guía rápida

Si quieres **entender el lenguaje**, lee `02` y `03`.
Si quieres **entender el compilador**, lee `01`, `05`, `07`, `08` y `09`.
Si quieres **usarlo**, mira el [`README.md`](../README.md).
