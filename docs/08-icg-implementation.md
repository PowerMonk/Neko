# Neko — Generacion de Codigo Intermedio

> Documento **08** de la serie [`docs/`](./00-index.md).
> Describe la fase de generacion de codigo intermedio (ICG) del
> compilador Neko: como se traducen las construcciones del AST a
> cuadruplos y triplos (notaciones clasicas de codigo de tres
> direcciones).
>
> Si vienes de la materia, esta fase se corresponde con la
> "Generacion de Codigo Intermedio" que cubre la unidad sobre
> representacion intermedia. La convencion didactica usada aqui es
> la de cuadruplos `(op, arg1, arg2, resultado)` y triplos
> `(op, arg1, arg2)` con resultados referenciados por indice
> posicional (Aho/Sethi/Ullman, "Dragon Book", §6.4).

---

## 1. Posicion en el pipeline

```
   .neko source
        |
        v
   +----------+
   |  lexer   |  -->  output/tokens.txt, output/symbols.txt, output/errors.txt
   +----+-----+
        | Vec<Token>
        v
   +----------+
   |  parser  |  -->  output/parse_tree.txt, output/parse_errors.txt
   +----+-----+
        | Program (AST)
        v
   +---------------+
   |   semantic    |  -->  output/semantic_errors.txt
   +-------+-------+
           | Program (AST validado)
           v
   +---------------+
   |   ICG         |  -->  output/quadruples.txt, output/triples.txt
   +---------------+
```

La fase ICG **solo corre** si las tres fases previas (lexico, sintactico,
semantico) produjeron cero errores. Esto evita emitir codigo sobre un AST
incompleto o un programa semanticamente invalido.

---

## 2. Estructura de `src/icg/`

```
src/icg/
├── mod.rs           <- declaraciones + re-exports
├── quadruple.rs     <- struct Quadruple { op, arg1, arg2, result }
├── triple.rs        <- struct Triple { op, arg1, arg2 }
├── emitter.rs       <- CodeEmitter: contadores de T_n y L_n, listas
└── generator.rs     <- ICGenerator: visitor sobre el AST
```

Cada archivo lleva un comentario de cabecera que explica que hace y por
que. El visitor (`generator.rs`) es el unico archivo que conoce el AST;
los demas son tipos de datos puros.

---

## 3. Las dos notaciones producidas

### 3.1 Cuadruplos

Tupla de 4 campos:

```
 ( operador , arg1 , arg2 , resultado )
```

| Campo      | Que contiene                                                                                  |
| ---------- | --------------------------------------------------------------------------------------------- |
| `operador` | Operador de Neko (`+`, `-`, `==`, ...) o pseudo-op (`GOTOF`, `GOTOV`, `GOTO`, `LABEL`, `PRINT`) |
| `arg1`     | Operando izquierdo: variable, literal, o temporal `T_n`                                       |
| `arg2`     | Operando derecho: idem, o `"_"` si no aplica                                                  |
| `resultado`| Destino: temporal `T_n` (para aritmetica), etiqueta `L_n` (para saltos), o `"_"`             |

**Ejemplo** — para `a + b * c`:

```
  0  (      *,      b,      c,     T1 )
  1  (      +,      a,     T1,     T2 )
```

### 3.2 Triplos

Tupla de 3 campos, con el **resultado implicito** por posicion:

```
 ( indice )  ( operador , arg1 , arg2 )
```

| Campo      | Que contiene                                                                                  |
| ---------- | --------------------------------------------------------------------------------------------- |
| `indice`   | Numero de orden (0, 1, 2, ...)                                                                 |
| `operador` | Igual que en cuadruplos                                                                        |
| `arg1`     | Operando o referencia a un triplo previo: `(0)`, `(1)`, ...                                     |
| `arg2`     | Idem                                                                                          |

**Mismo ejemplo** — para `a + b * c`:

```
 (  0)  (      *,      b,      c )
 (  1)  (      +,      a,     (0) )
```

En esta implementacion el campo `arg` de los triplos es la cadena cruda
(`b`, `c`, `a`, `(0)`); la referencia `(0)` aparece cuando un argumento
es a su vez el resultado de otro triplo.

---

## 4. Reglas de traduccion

### 4.1 Convenciones

| Simbolo | Significado                                                |
| ------- | ---------------------------------------------------------- |
| `T1`, `T2`, ... | Temporales. Indice base 1.                     |
| `L1`, `L2`, ... | Etiquetas de salto. Indice base 1.            |
| `GOTOF`         | Salto si falso (equivalente a `if not T goto L`). |
| `GOTOV`         | Salto si verdadero (equivalente a `if T goto L`). |
| `GOTO`          | Salto incondicional.                                  |
| `LABEL`         | Marca una etiqueta.                                   |
| `PRINT`         | Llamada a `meow` en Neko.                            |

### 4.2 Que se traduce y que no

| Construccion Neko    | Cuadruplos emitidos                                       |
| -------------------- | --------------------------------------------------------- |
| `a + b`              | `(+, a, b, T1)`                                           |
| `a + b + c`          | `(+, a, b, T1)` `(+, T1, c, T2)`                          |
| `a * b`              | `(*, a, b, T1)`                                           |
| `-x`                 | `(-, x, _, T1)`                                           |
| `!x`                 | `(!, x, _, T1)`                                           |
| `a > b`              | `(>, a, b, T1)`                                           |
| `if c { A } else { B }` | `(GOTOF, c, _, L_else)` `A` `(GOTO, _, _, L_end)` `LABEL L_else` `B` `LABEL L_end` |
| `if c { A }` (sin else) | `(GOTOF, c, _, L_end)` `A` `LABEL L_end`             |
| `match x { p1 => A, p2 => B, _ => D }` | `==, GOTOV` por brazo no-default; LABEL por brazo; `GOTO L_end` entre brazos; LABEL final |
| `meow(x)`            | `(PRINT, x, _, _)`                                        |
| `nyan x = expr`      | SOLO los cuadruplos de `expr`. El `nyan x =` **no emite**.|
| `fn neko { ... }`    | SOLO los cuadruplos del cuerpo. El `fn neko` **no emite**.|
| `{` `}`              | No emiten nada.                                           |

---

## 5. Estructura de control: `if/else`

Patron canonico para `if <cond> <then> else <else>`:

```
   (GOTOF, cond, _, L_else)    ; si la cond es falsa, salta al else
   <cuadruplos del then>
   (GOTO,   _,    _, L_end)    ; al terminar el then, salta al final
   (LABEL,  _,    _, L_else)   ; marca donde empieza el else
   <cuadruplos del else>
   (LABEL,  _,    _, L_end)    ; marca el final
```

Sin `else`, se omite la parte de `L_else` y la condicion falsa salta
directamente a `L_end`.

---

## 6. Estructura de control: `match`

Patron para `match x { p1 => A, p2 => B, _ => D }` (donde `_` es el
default, obligatorio en Neko):

```
   (==,     x, p1, T1)          ; test del primer brazo
   (GOTOV,   T1, _, L_arm1)     ; si T1 es verdadero, salta al cuerpo
   (==,     x, p2, T2)          ; test del segundo brazo
   (GOTOV,   T2, _, L_arm2)
   (LABEL,  _,   _, L_arm1)     ; cuerpo del primer brazo
   <cuadruplos del cuerpo A>
   (GOTO,   _,   _, L_end)
   (LABEL,  _,   _, L_arm2)     ; cuerpo del segundo brazo
   <cuadruplos del cuerpo B>
   (GOTO,   _,   _, L_end)
   (LABEL,  _,   _, L_default)  ; cuerpo del default
   <cuadruplos del cuerpo D>
   (LABEL,  _,   _, L_end)      ; fin del match
```

Observaciones importantes:

1. **El default no emite test.** Si los brazos anteriores no
   coincidieron, el control cae naturalmente a `L_default`.
2. **Cada brazo no-default emite `GOTO L_end` al final** para que la
   ejecucion no caiga en el siguiente cuerpo.
3. **El default no emite `GOTO L_end`** porque su final ES el final.

---

## 7. El `Value`: como devuelve cada expresion su resultado

Cada sub-expresion del AST devuelve un `Value` (enum en
`generator.rs`):

| Variante                 | Significado                                                       |
| ------------------------ | ----------------------------------------------------------------- |
| `Value::Operand(s)`      | El resultado ya esta en `s` (variable, literal, o `T_n`). No emite.|
| `Value::OperandUnderscore` | La expresion es de control de flujo; no tiene unico resultado.   |

Cuando una expresion aritmetica ve dos operandos, pide un temporal
nuevo al `CodeEmitter` y emite el cuadruplo. El temporal se devuelve
como `Value::Operand("T_n")` para que el padre lo use como argumento.

---

## 8. CodeEmitter: contadores centralizados

Para evitar colisiones de nombres entre temporales, el
`CodeEmitter` lleva un contador global que se incrementa con
`next_temp()` y `next_label()`. Esto permite que el codigo generado
sea determinista y reproducible (la misma entrada produce el mismo
codigo).

---

## 9. Forma de los archivos de salida

Cada cuadruplo y triplo se imprime en una linea, con columnas
alineadas a 6 caracteres. Ejemplo real de `output/quadruples.txt`
para `examples/icg-tests/icg-ok.neko`:

```
  0  (      *,      b,      c,     T1 )
  1  (      +,      a,     T1,     T2 )
  2  (     ==,  score,    100,     T3 )
  3  (  GOTOV,     T3,      _,     L2 )
  4  (     ==,  score,     90,     T4 )
  5  (  GOTOV,     T4,      _,     L3 )
  6  (  LABEL,      _,      _,     L2 )
  7  (   GOTO,      _,      _,     L1 )
  8  (  LABEL,      _,      _,     L3 )
  9  (   GOTO,      _,      _,     L1 )
 10  (  LABEL,      _,      _,     L4 )
 11  (  LABEL,      _,      _,     L1 )
 12  (      >,      x,     30,     T5 )
 13  (  GOTOF, condicion,      _,     L5 )
 14  (   GOTO,      _,      _,     L6 )
 15  (  LABEL,      _,      _,     L5 )
 16  (  LABEL,      _,      _,     L6 )
 17  (  PRINT,      x,      _,      _ )
 18  (  PRINT,  final,      _,      _ )
```

`output/triples.txt` tiene el mismo formato pero sin el campo
`resultado` y con un numero de orden a la izquierda.

---

## 10. Resumen en terminal

Cuando todo sale bien, la terminal muestra:

```
OK — 85 tokens, 8 symbols, parse tree built with 0 syntactic errors,
     semantic analysis: 0 errors, intermediate code: 19 quadruples, 13 triples.
```

Cuando hay errores en alguna fase previa:

```
Errors found — lexical: 1, syntactic: 0, semantic: 0. See output/errors.txt,
output/parse_errors.txt, output/semantic_errors.txt.
```

`output/quadruples.txt` y `output/triples.txt` se escriben igual con
texto marcador `(skipped: errors in prior phases)`.

---

## 11. Decisiones de diseno

### 11.1 Una sola pasada genera ambos formatos

El `CodeEmitter` lleva DOS listas (cuadruplos y triplos) y DOS
contadores. Cada `emit_quad` y `emit_triple` agrega a su lista
respectiva. Asi evitamos recorrer el programa dos veces.

### 11.2 El ICG NO valida semantica

La convencion del proyecto es: "si llegaste aqui, el programa ya es
correcto". El ICG no verifica tipos ni unicidad. Si los temporales
generados quedan sin usar (por ejemplo, el resultado de una
comparacion que se asigna a una variable pero la variable luego no
se usa), eso es problema del programador, no del ICG.

### 11.3 Los temporales T_n y las etiquetas L_n son globales al programa

No hay `T_n` por bloque ni `L_n` por funcion. Esto refleja el
modelo de ejecucion plano del codigo intermedio. Si en una
iteracion futura se agregan funciones con parametros, habra que
revisar esta decision.

### 11.4 Los triplos NO usan referencias explicitas `(0)`, `(1)`, ...

Por simplicidad, el argumento de un triplo es la cadena cruda
(`T1`, `a`, `b`). El "resultado implicito" se materializa
visualmente como el orden de las lineas, que es la convencion
usual cuando el codigo se muestra linea por linea. Si se prefiere
la notacion con referencias `(0)`, basta cambiar la linea
`emit_triple` en el `CodeEmitter` para anteponer `format!("({})",
indice_actual)` cuando se conozca la posicion.

---

## 12. Cobertura de tests

15 pruebas unitarias en `src/icg/tests.rs`:

| Categoria              | Tests                                                                                |
| ---------------------- | ------------------------------------------------------------------------------------ |
| **CodeEmitter**        | `emitter_temps_start_at_t1`, `emitter_labels_start_at_l1`, `emitter_triple_index_is_zero_based` |
| **Render**             | `quadruple_render_columns_aligned`, `triple_render_includes_index`                   |
| **Expresiones**        | `empty_program_emits_nothing`, `var_decl_does_not_emit_quads`, `simple_addition_emits_one_quad`, `multiplication_chain_emits_correct_order`, `comparison_emits_op_quad`, `print_stmt_emits_print_quad`, `both_quadruples_and_triples_emitted` |
| **Control de flujo**   | `if_else_emits_gotof_goto_labels`, `match_with_three_arms_emits_correct_structure`  |
| **Estructura**         | `fn_decl_emits_only_body_quads`                                                      |

Mas las 8 de parser y 16 de semantica, total **39 pruebas pasando**:

```
$ cargo test
test result: ok. 39 passed; 0 failed
```

---

## 13. Programas de ejemplo

```
examples/
+-- basic.neko                      <- canonico (sin cambios)
+-- broken.neko                     <- errores sintacticos
+-- operators.neko                  <- cadenas de operadores
+-- lex-error.neko                  <- nuevo: 1 error detectable por el lexer
+-- syn-error.neko                  <- nuevo: 1 error detectable por el parser
+-- sem-error.neko                  <- nuevo: 1 error detectable por el semantico
+-- sem-tests/                       <- 8 programas de prueba del analizador semantico
+-- icg-tests/icg-ok.neko           <- nuevo: entrada limpia para demostrar cuadruplos/triplos
```

Para la demonstracion en video, los archivos `lex-error.neko`,
`syn-error.neko`, `sem-error.neko` y `icg-tests/icg-ok.neko`
ejercitan una fase distinta cada uno.

---

## 14. Limitaciones conocidas

- **Sin optimizacion.** El ICG no hace constant folding, copy
  propagation, ni nada parecido. Cada operacion genera un temporal.
- **Sin manejo de errores de ejecucion.** Division por cero,
  desbordamiento, etc. son responsabilidad de la fase de ejecucion.
- **Triplos sin referencia circular.** No se usa la notacion
  `(0)`, `(1)` para referenciar triplos anteriores; el "resultado"
  es siempre la cadena cruda del temporal. Ver §11.4.
- **Sin funciones con parametros.** El unico "fn" de Neko es `fn
  neko { ... }`, asi que no hay manejo de frames, paso de
  argumentos, ni nada relacionado.
