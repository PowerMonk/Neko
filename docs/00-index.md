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

## Convención de nombres

Cada archivo lleva un prefijo numérico de dos dígitos para forzar el
orden de lectura en listados de directorio y en hipervínculos. El
número refleja el orden lógico del flujo del compilador:

```
01 (lexer)  →  02 (grammar)  →  03 (design)  →  04 (parse tree)  →  05 (parser)
   entrada        especificación     justificaciones      ejemplo visual      implementación
```

Documentos que se añadan después deben seguir la numeración: `06-...`,
`07-...`, etc.
