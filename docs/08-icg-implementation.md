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
| `if c { A } else { B }` | `(GOTOF, c, _, L_else)` `A` `(=, A_val, _, T_res)` `(GOTO, _, _, L_end)` `LABEL L_else` `B` `(=, B_val, _, T_res)` `LABEL L_end` |
| `if c { A }` (sin else) | `(=, 0, _, T_res)` `(GOTOF, c, _, L_end)` `A` `(=, A_val, _, T_res)` `LABEL L_end` |
| `match x { p1 => A, p2 => B, _ => D }` | `==`, `GOTOV` por brazo no-default; `GOTO` al default; `LABEL` por brazo; `(=, <valor del brazo>, _, T_res)` en cada brazo; `GOTO L_end` entre brazos; LABEL final |
| `meow(x)`            | `(PRINT, x, _, _)`                                        |
| `nyan x = expr`      | Los cuadruplos de `expr` **más** el cuadruplo de asignacion `(=, <valor>, _, x)`. La palabra clave `nyan` **nunca** aparece. Ver §5. |
| `fn neko { ... }`    | SOLO los cuadruplos del cuerpo. El `fn neko` **no emite**.|
| `{` `}`              | No emiten nada.                                           |

---

## 5. El cuadruplo de asignacion: por que existe

Esta seccion justifica una decision de diseno central: la aparicion del
cuadruplo `(=, <origen>, _, <variable>)` en la tabla de cuadruplos.

### 5.1 La regla

En Neko, `nyan x = expr` produce dos cosas distintas, y la distinction
es importante:

| Parte de `nyan x = expr`      | ¿Genera cuadruplo? | Justificacion                          |
|------------------------------|--------------------|----------------------------------------|
| La **declaracion** (el `nyan`, el nombre, el tipo) | ❌ No | Como `int a;` en C, que no genera codigo. No existe ningun cuadruplo `DECLARE`. |
| La **asignacion** (el `=` con su valor)         | ✅ Si  | Como `a = 5;` en C, que SI genera codigo. |

Es exactamente la distincion de C:

```c
int a;          // declaracion pura -> NO genera codigo
a = 5;          // asignacion       -> SI genera codigo
```

En Neko `nyan a = 5` es una declaracion **y** una asignacion
combinadas. Aplicamos la misma regla de C: la declaracion no se
traduce, la asignacion si.

**Referencia:** Aho, Sethi & Ullman ("Dragon Book", *Compilers:
Principles, Techniques, and Tools*), §6.7 *Translation of Statements*.
La traduccion de `x = y + z` produce:

```
t1 = y + z
x  = t1
```

que en notacion de cuadruplos es:

```
( +, y, z, t1 )
( =, t1, _, x  )
```

### 5.2 Reconciliacion con las notas de la maestra

Las notas de la maestra para esta fase dicen (aproximadamente):

> **Nota 1:** "las declaraciones de variables NO van en los cuadruplos"
> **Nota 2:** "no podiamos tomar la tabla de cuadruplos para asignar las variables"
> **Nota 3:** "las direcciones de los datos se obtendrian de la tabla de simbolos"

Las tres se cumplen, porque hablan de cosas distintas:

| Nota | Quien la cumple | Como |
|------|-----------------|------|
| **Nota 1** — las declaraciones no van en los cuadruplos | El **cuadruplo** | No existe ningun cuadruplo `DECLARE`. En la tabla de cuadruplos **nunca aparece la palabra `nyan`** ni el nombre de una "declaracion": solo operaciones y asignaciones. |
| **Nota 2** — la tabla de cuadruplos no se usa para *asignar* las variables | La **tabla de simbolos** | El cuadruplo `(=, T1, _, score)` **no dice donde vive** `score`. Dice "copia el valor de T1 al destino llamado score". La direccion de `score` la resuelve la **tabla de simbolos**, que es lo que dice la Nota 3. |
| **Nota 3** — las direcciones vienen de la tabla de simbolos | La **tabla de simbolos** | Ver §9 de [`09-codegen-implementation.md`](./09-codegen-implementation.md). |

**La distincion clave, para explicar en la defensa:**

```
Tabla de SIMBOLOS   ->  DONDE vive cada variable (seccion, offset, tamano, tipo)
Tabla de CUADRUPLOS ->  QUE operaciones hace el programa, y en que orden
```

Son dos tablas complementarias. La tabla de cuadruplos **nunca**
almacena el valor de una variable ni su direccion: solo *opera sobre*
ella usando un nombre que la tabla de simbolos traduce a una direccion.
Por eso el generador de codigo necesita **las dos**: la de cuadruplos
(que hacer) y la de simbolos (donde).

### 5.3 Por que no puede hacerse en otra parte

Una alternativa era que la asignacion la hiciera el **generador de
codigo** en vez del ICG, para que la tabla de cuadruplos quedara "limpia".
Se descarta por una razon tecnica:

La tabla de cuadruplos es una **lista plana**. Sin una asignacion
explicita, no hay forma de saber que el temporal `T1` (resultado de
`a + b`) debe terminar en `x`. El ICG es el unico punto donde se sabe
eso, porque es donde se ve el `nyan x = ...`.

La otra alternativa — que el generador de codigo recibiera el AST
*ademas* de los cuadruplos y los alineara en paralelo — funciona, pero
obliga a **duplicar la logica de "cuantos cuadruplos genera cada
expresion"** en dos modulos, y a acoplar dos fases que deben ser
independientes.

La practica estandar resuelve esto en el ICG, que es donde corresponde.

### 5.4 El antes y el despues, sobre `examples/basic.neko`

**ANTES** (13 cuadruplos) — las variables nunca reciben un valor:

```
  0  (     ==,  score,    100,     T1 )
  1  (  GOTOV,     T1,      _,     L2 )
  2  (     ==,  score,     90,     T2 )
  3  (  GOTOV,     T2,      _,     L3 )
  4  (  LABEL,      _,      _,     L2 )
  5  (   GOTO,      _,      _,     L1 )
  6  (  LABEL,      _,      _,     L3 )
  7  (   GOTO,      _,      _,     L1 )
  8  (  LABEL,      _,      _,     L4 )
  9  (  LABEL,      _,      _,     L1 )
 10  (  PRINT, result,      _,      _ )
 11  (  PRINT, holamundo,      _,      _ )
 12  (  PRINT, banana,      _,      _ )
```

Con este listado, `meow(result)` imprimiria basura: `result` nunca
recibio el valor de ninguna rama. El programa **no funcionaba**.

**DESPUES** (21 cuadruplos) — cada variable recibe su valor:

```
  0  (      =,     90,      _,    score )
  1  (      =,     90,      _,   banana )
  2  (      =, "Hola mundo desde Neko", _, holamundo )
  3  (     ==,  score,    100,     T2 )
  4  (  GOTOV,     T2,      _,     L2 )
  5  (     ==,  score,     90,     T3 )
  6  (  GOTOV,     T3,      _,     L3 )
  7  (   GOTO,      _,      _,     L4 )
  8  (  LABEL,      _,      _,     L2 )
  9  (      =, "Perfect",      _,     T1 )
 10  (   GOTO,      _,      _,     L1 )
 11  (  LABEL,      _,      _,     L3 )
 12  (      =, "Excellent",      _,     T1 )
 13  (   GOTO,      _,      _,     L1 )
 14  (  LABEL,      _,      _,     L4 )
 15  (      =, "Fail",      _,     T1 )
 16  (  LABEL,      _,      _,     L1 )
 17  (      =,     T1,      _, result )
 18  (  PRINT, result,      _,      _ )
 19  (  PRINT, holamundo,      _,      _ )
 20  (  PRINT, banana,      _,      _ )
```

Ahora `result` recibe el valor de la rama del `match` que se ejecuto, y
`holamundo` recibe la direccion de su cadena. El programa **funciona**:

```
$ cargo run -- examples/basic.neko
--- ejecutando el programa compilado ---
ExcellentHola mundo desde Neko90
```

### 5.5 `if` y `match` como expresiones con valor

El mismo principio aplica a las construcciones de control de flujo. En
Neko `if` y `match` son **expresiones**: producen un valor. Por eso el
ICG les reserva un temporal de resultado y le asigna el valor de la
rama que se ejecuto.

Para `nyan r = if c { 1 } else { 2 }`:

```
  ( >, a, b, T1 )            <- la condicion
  ( GOTOF, T1, _, L1 )       <- si es falsa, salta al else
  ( =, 1, _, T2 )            <- el valor de la rama then
  ( GOTO, _, _, L3 )
  ( LABEL, _, _, L1 )        <- empieza el else
  ( =, 2, _, T2 )            <- el valor de la rama else
  ( LABEL, _, _, L3 )        <- fin
  ( =, T2, _, r )            <- r toma el valor
```

Sin `else`, el temporal de resultado se pre-inicializa a `0` (un puntero
nulo para cadenas, cero para enteros y booleanos) para que el camino
"falso" tenga un valor definido.

Todos los brazos de un `match` escriben en el **mismo** temporal de
resultado, de modo que el valor del `match` esta bien definido
independientemente de que brazo se haya ejecutado.

---

## 6. Estructura de control: `if/else`

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

## 7. Estructura de control: `match`

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

## 8. El `Value`: como devuelve cada expresion su resultado

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

## 9. CodeEmitter: contadores centralizados

Para evitar colisiones de nombres entre temporales, el
`CodeEmitter` lleva un contador global que se incrementa con
`next_temp()` y `next_label()`. Esto permite que el codigo generado
sea determinista y reproducible (la misma entrada produce el mismo
codigo).

---

## 10. Forma de los archivos de salida

Cada cuadruplo y triplo se imprime en una linea, con columnas
alineadas a 6 caracteres. Ejemplo real de `output/quadruples.txt`
para `examples/icg-tests/icg-ok.neko`:

```
  0  (      =,     10,      _,      a )
  1  (      =,      3,      _,      b )
  2  (      =,      2,      _,      c )
  3  (      *,      b,      c,     T1 )
  4  (      +,      a,     T1,     T2 )
  5  (      =,     T2,      _,      x )
  6  (      =,     90,      _,  score )
  7  (     ==,  score,    100,     T4 )
  8  (  GOTOV,     T4,      _,     L2 )
  9  (     ==,  score,     90,     T5 )
 10  (  GOTOV,     T5,      _,     L3 )
 11  (   GOTO,      _,      _,     L4 )
 12  (  LABEL,      _,      _,     L2 )
 13  (      =, "Perfect",      _,     T3 )
 14  (   GOTO,      _,      _,     L1 )
 15  (  LABEL,      _,      _,     L3 )
 16  (      =, "Excellent",      _,     T3 )
 17  (   GOTO,      _,      _,     L1 )
 18  (  LABEL,      _,      _,     L4 )
 19  (      =, "Fail",      _,     T3 )
 20  (  LABEL,      _,      _,     L1 )
 21  (      =,     T3,      _, result )
 22  (      >,      x,     30,     T6 )
 23  (      =,     T6,      _, condicion )
 24  (  GOTOF, condicion,      _,     L5 )
 25  (      =, result,      _,     T7 )
 26  (   GOTO,      _,      _,     L6 )
 27  (  LABEL,      _,      _,     L5 )
 28  (      =, "Bajo",      _,     T7 )
 29  (  LABEL,      _,      _,     L6 )
 30  (      =,     T7,      _,  final )
 31  (  PRINT,      x,      _,      _ )
 32  (  PRINT,  final,      _,      _ )
```

Observaciones sobre este listado:

- Las lineas `0`, `1`, `2`, `5`, `6`, `21`, `23` y `30` son los
  cuadruplos de **asignacion** que explican la §5.
- `T3` es el temporal de resultado del `match`: los tres brazos
  escriben en el mismo temporal (lineas 13, 16 y 19).
- `T7` es el temporal de resultado del `if`: las dos ramas escriben en
  el mismo temporal (lineas 25 y 28).
- El `GOTO L4` de la linea 11 es el "else" de la cadena de
  comparaciones del `match`: si ningun patron coincide, el control
  salta al brazo por defecto.

`output/triples.txt` tiene el mismo formato pero sin el campo
`resultado` y con un numero de orden a la izquierda.

---

## 11. Resumen en terminal

Cuando todo sale bien, la terminal muestra:

```
OK — 85 tokens, 8 symbols, parse tree built with 0 syntactic errors,
     semantic analysis: 0 errors, intermediate code: 33 quadruples, 33 triples,
     assembly: output/program.asm.
```

Cuando hay errores en alguna fase previa:

```
Errors found — lexical: 1, syntactic: 0, semantic: 0. See output/errors.txt,
output/parse_errors.txt, output/semantic_errors.txt.
```

`output/quadruples.txt` y `output/triples.txt` se escriben igual con
texto marcador `(skipped: errors in prior phases)`.

---

## 12. Decisiones de diseno

### 12.1 Una sola pasada genera ambos formatos

El `CodeEmitter` lleva DOS listas (cuadruplos y triplos) y DOS
contadores. Cada `emit_quad` y `emit_triple` agrega a su lista
respectiva. Asi evitamos recorrer el programa dos veces.

La correspondencia es **1 a 1**: por cada cuadruplo hay exactamente un
triplo en la misma posicion relativa. Esto esta verificado por el test
`quadruples_and_triples_stay_in_sync_for_match`.

### 12.2 El ICG NO valida semantica

La convencion del proyecto es: "si llegaste aqui, el programa ya es
correcto". El ICG no verifica tipos ni unicidad. Si los temporales
generados quedan sin usar (por ejemplo, el resultado de una
comparacion que se asigna a una variable pero la variable luego no
se usa), eso es problema del programador, no del ICG.

### 12.3 Los temporales T_n y las etiquetas L_n son globales al programa

No hay `T_n` por bloque ni `L_n` por funcion. Esto refleja el
modelo de ejecucion plano del codigo intermedio. Si en una
iteracion futura se agregan funciones con parametros, habra que
revisar esta decision.

### 12.4 Los triplos NO usan referencias explicitas `(0)`, `(1)`, ...

Por simplicidad, el argumento de un triplo es la cadena cruda
(`T1`, `a`, `b`). El "resultado implicito" se materializa
visualmente como el orden de las lineas, que es la convencion
usual cuando el codigo se muestra linea por linea. Si se prefiere
la notacion con referencias `(0)`, basta cambiar la linea
`emit_triple` en el `CodeEmitter` para anteponer `format!("({})",
indice_actual)` cuando se conozca la posicion.

### 12.5 El cuadruplo de asignacion vive en el ICG

Ver la §5 completa. En resumen: la declaracion (`nyan`) no genera nada
—igual que `int a;` en C— pero la asignacion del valor si, porque sin
ella el valor nunca llegaria a la variable. Es la traduccion estandar
del Dragon Book §6.7.

### 12.6 `GeneratedCode` conserva los cuadruplos estructurados

Ademas del texto renderizado, `GeneratedCode` guarda el
`Vec<Quadruple>` y el `Vec<Triple>` originales. El generador de codigo
ensamblador (`src/codegen/`) los consume directamente, en vez de
volver a parsear el texto de `output/quadruples.txt`. Ver
[`09-codegen-implementation.md`](./09-codegen-implementation.md).

---

## 13. Cobertura de tests

20 pruebas unitarias en `src/icg/tests.rs`:

| Categoria              | Tests                                                                                |
| ---------------------- | ------------------------------------------------------------------------------------ |
| **CodeEmitter**        | `emitter_temps_start_at_t1`, `emitter_labels_start_at_l1`, `emitter_triple_index_is_zero_based` |
| **Render**             | `quadruple_render_columns_aligned`, `triple_render_includes_index`                   |
| **Expresiones**        | `empty_program_emits_nothing`, `simple_addition_emits_op_and_assignment`, `multiplication_chain_emits_correct_order`, `comparison_emits_op_quad`, `print_stmt_emits_print_quad`, `both_quadruples_and_triples_emitted` |
| **Asignacion** (§5)    | `var_decl_emits_assignment_quad` (verifica ademas que `nyan` NO aparezca en la tabla) |
| **Control de flujo**   | `if_else_emits_gotof_goto_labels`, `if_without_else_initialises_result_to_zero`, `match_with_three_arms_emits_correct_structure`, `match_result_temp_is_shared_by_all_arms` |
| **Estructura**         | `fn_decl_emits_only_body_quads`, `quadruples_and_triples_stay_in_sync_for_match`    |

Mas las 8 de parser, 16 de semantica y 34 de codegen, total
**78 pruebas pasando**:

```
$ cargo test
test result: ok. 78 passed; 0 failed
```

---

## 14. Programas de ejemplo

```
examples/
+-- basic.neko                      <- canonico (sin cambios)
+-- broken.neko                     <- errores sintacticos
+-- operators.neko                  <- cadenas de operadores
+-- lex-error.neko                  <- 1 error detectable por el lexer
+-- syn-error.neko                  <- 1 error detectable por el parser
+-- sem-error.neko                  <- 1 error detectable por el semantico
+-- sem-tests/                       <- 8 programas de prueba del analizador semantico
+-- icg-tests/icg-ok.neko           <- entrada limpia: aritmetica, if/else y match de 3 brazos
+-- icg-tests/icg-if-true.neko      <- la rama verdadera del if/else
```

Para la demostracion en video, los archivos `lex-error.neko`,
`syn-error.neko`, `sem-error.neko` y `icg-tests/icg-ok.neko`
ejercitan una fase distinta cada uno.

---

## 15. Limitaciones conocidas

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
