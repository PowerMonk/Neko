# Neko — Convenciones Semánticas

> Documento **06** de la serie [`docs/`](./00-index.md).
> Define las **comprobaciones estáticas** que la fase de análisis
> semántico del compilador se compromete a realizar. Las
> comprobaciones se efectúan sobre el **árbol sintáctico con
> anotaciones semánticas** (un AST al que cada nodo `Expr` lleva
> asociado un tipo inferido).
>
> Los mensajes de error que produce el compilador están escritos en
> inglés (formato `linea:col -> mensaje` consistente con el lexer y
> el parser en `output/`); el resto del documento está en español
> porque la materia se cursa en español.

---

## Contexto teórico

**Comprobaciones estáticas** son las que el compilador realiza sobre
el código fuente antes de ejecutar el programa. Son responsabilidad
del compilador. Existen cuatro categorías clásicas (Aho/Sethi/Ullman),
que son las que este documento desarrolla:

1. Comprobación de tipos
2. Comprobaciones de flujo de control
3. Comprobaciones de unicidad
4. Comprobaciones relacionadas con nombres

**Comprobaciones dinámicas** son las que se realizan durante la
ejecución del programa (por ejemplo, división entre cero, acceso a
índices fuera de rango). Son responsabilidad del programador (típicamente
del lenguaje de programación o del runtime) y NO se incluyen en este
documento.

---

## Resumen de las cuatro comprobaciones estáticas de Neko

| #  | Categoría                                  | Cuándo se ejecuta                  |
| -- | ------------------------------------------ | ---------------------------------- |
| 1  | Comprobación de tipos                      | Pasada bottom-up sobre el AST      |
| 2  | Comprobaciones de flujo de control         | Al visitar cada nodo de control    |
| 3  | Comprobaciones de unicidad                 | Cada vez que se inserta un símbolo |
| 4  | Comprobaciones relacionadas con nombres    | Al cierre de cada bloque con nombre |

Para cada categoría se aplica sobre los nodos AST que correspondan
según la gramática de [`02-grammar.md`](./02-grammar.md).

---

## 1. Comprobación de tipos

**Regla:** Neko realiza inferencia estática de tipos: cada expresión
tiene un tipo inferido a partir de sus sub-expresiones, y ese tipo se
compara con el tipo esperado por el contexto donde la expresión
aparece.

### Tipos básicos y construidos

Neko trabaja con dos clases de tipos:

| Clase          | Tipos en esta iteración                                |
| -------------- | ------------------------------------------------------ |
| **Básicos**    | `int`, `string`, `bool`                                |
| **Construidos**| `fn` (tipo función, sin parámetros en esta iteración)  |

En esta primera iteración no hay arrays, mapas, tuplas ni registros;
esos se añadirán como tipos construidos en iteraciones futuras.

### Reglas de inferencia por nodo AST

| Forma del AST              | Tipo inferido                                           |
| -------------------------- | ------------------------------------------------------- |
| `Literal::Int(_)`          | `int`                                                   |
| `Literal::Str(_)`          | `string`                                                |
| `Literal::True`/`False`    | `bool`                                                  |
| `Literal::Ident(name)`     | El tipo declarado en `nyan name = …`                    |
| `Literal::Grouped(inner)`  | El tipo de `inner`                                      |
| `Chain<AddOpChain>`        | `int` (operadores de suma/resta son int)                |
| `Chain<MulOpChain>`        | `int` (operadores de mult/div/mod son int)               |
| `Chain<LogicalOpChain>`    | `bool`                                                  |
| `Expr::Compare { op, … }`  | `bool`, siempre                                         |
| `Expr::Negate { operand }` | `int` si el operando es `int`; error en otro caso       |
| `Expr::Not { operand }`    | `bool` si el operando es `bool`; error en otro caso     |
| `Expr::If { … }`           | El tipo de `then` debe coincidir con el de `else`       |
| `Expr::Match { arms, … }`  | Todas las ramas deben devolver el mismo tipo            |

### Reglas de verificación contextual (compatibilidad)

| Contexto                            | Requisito                                             |
| ----------------------------------- | ----------------------------------------------------- |
| RHS de `nyan name = <expr>`         | Cualquier tipo permitido                              |
| Condición de `if`                   | Debe ser `bool`                                       |
| Operandos de `+ - * / %`            | Ambos `int`                                           |
| Operandos de `== != < > <= >=`      | Mismo tipo entre sí                                   |
| Operandos de `&& \|\|`              | Ambos `bool`                                          |
| Operando de `!`                     | `bool`                                                |
| Cuerpo de `fn neko { … }`           | El último statement del bloque define el tipo de retorno |

### Mensajes de error canónicos

| Mensaje                                                                         | Cuándo                                              |
| ------------------------------------------------------------------------------- | --------------------------------------------------- |
| `type mismatch: expected <T1>, found <T2>`                                      | Un operando no satisface el tipo esperado           |
| `type mismatch: condition of 'if' must be bool, found <T>`                      | `if … { … }` donde la condición no es `bool`        |
| `type mismatch: branches of 'if' return different types (<T1> vs <T2>)`        | `then` y `else` con tipos distintos                  |
| `type mismatch: arms of 'match' return different types (<T1> vs <T2>)`         | Distintos brazos del `match` retornan tipos distintos |
| `cannot negate non-int value of type <T>`                                       | `-x` con `x:string` o `x:bool`                      |
| `cannot apply '!' to non-bool value of type <T>`                                | `!x` con `x:int` o `x:string`                       |

> Los marcadores `<T>`, `<T1>`, `<T2>` se sustituyen por el nombre del
> tipo real al emitir el error.

---

## 2. Comprobaciones de flujo de control

**Regla:** toda instrucción que transfiere el control a otro bloque de
código debe tener un destino válido. Si el destino es inexistente o
condicionalmente inalcanzable, el compilador emite un error estático.

En Neko esta primera iteración las transferencias de control son
limitadas:

| Instrucción         | Destino de la transferencia                  | Comprobación                                          |
| ------------------- | -------------------------------------------- | ----------------------------------------------------- |
| `if <expr> { … }`   | Salto al `then` y, si existe, al `else`      | Ver §1: `then` y `else` deben devolver el mismo tipo  |
| `match <expr> { … }`| Salto al brazo cuyo patrón coincida          | Ver §3: todo `match` debe incluir un brazo `_`        |
| `fn neko { … }`     | Llamada implícita al cuerpo                  | El bloque es accesible (no es una comprobación adicional) |

**Nota para iteraciones futuras:** cuando se agregue `break`,
`continue`, `return`, `while`, `for`, o llamadas explícitas a
funciones con parámetros, esta sección se ampliará con:

- `break` y `continue` sólo dentro de estructuras de bucle.
- `return` con tipo compatible con el tipo de retorno declarado.
- Cada brazo de `match` debe poder asignarse al destino.
- Las etiquetas mencionadas en §4 (no aplicables todavía).

### Mensaje de error canónico

| Mensaje                                                       | Cuándo                                              |
| ------------------------------------------------------------- | --------------------------------------------------- |
| `non-exhaustive match: default arm ('_') is required`         | `match` sin brazo cuyo patrón sea `Wildcard`         |

(Mismo mensaje que §3 — la exhaustividad es a la vez una comprobación
de unicidad del "camino por defecto" y una comprobación de flujo de
control porque garantiza que la transferencia tiene adónde ir.)

---

## 3. Comprobaciones de unicidad

**Regla:** cada elemento debe ser declarado o definido **exactamente
una vez** dentro del ámbito donde aplica. Una declaración duplicada
es un error estático.

> Nota sobre polimorfismo: algunos lenguajes rompen esta regla cuando
> aplican **polimorfismo de sobrecarga** (varias funciones con el
> mismo nombre pero distinta firma). Neko **no soporta sobrecarga**
> en esta iteración, así que la regla se mantiene estricta.

Aplica a:

- **Variables** declaradas con `nyan`: una sola vez por ámbito.
- **Funciones** declaradas con `fn neko`: una sola vez por programa.
  (En Neko sólo se permite una función `neko` por programa; múltiples
  declaraciones son error.)
- **Brazos de `match`**: ver §2 y §4. Cada brazo debe tener un patrón
  único (no se repite el mismo literal dos veces).
- **Etiquetas** en bloques con nombre: ver §4. Cuando se añadan
  (futuro), no se podrá repetir la misma etiqueta en el mismo ámbito.

### Sin shadowing (decisión de proyecto)

Combinada con la regla anterior, Neko prohíbe **shadowing** (declarar
en un ámbito interno un nombre ya declarado en un ámbito padre):

```neko
fn neko {
    nyan x = 1
    if true {
        nyan x = 2   # ERROR: shadowing no permitido
    }
}
```

### Mensajes de error canónicos

| Mensaje                                                                     | Cuándo                                                 |
| --------------------------------------------------------------------------- | ------------------------------------------------------ |
| `redeclaration of 'x' in the same scope`                                    | Dos `nyan x` en el mismo bloque                        |
| `'x' shadows an outer declaration (Neko disallows shadowing)`              | `nyan x` en un ámbito donde `x` ya existe              |
| `redeclaration of function 'neko'`                                          | Más de un `fn neko { … }` en el mismo programa          |
| `duplicate match arm: pattern already used in a previous arm`               | Dos brazos de `match` con el mismo patrón              |

---

## 4. Comprobaciones relacionadas con nombres

**Regla:** cuando un bloque de código deba delimitarse con una
etiqueta específica (por ejemplo, en HTML: `<etiqueta>…</etiqueta>`),
la etiqueta debe existir, estar bien escrita y ser única en su
ámbito.

### Aplicación a Neko

En esta primera iteración de Neko **no hay bloques con etiqueta
explícita**. La categoría se documenta para completud con la
clasificación de la materia. Las futuras construcciones que sí
usarán etiquetas (y，届时 activarán esta comprobación) son:

- **Plantillas literales** tipo HTML embebidas (futuro).
- **Bloques con nombre** que aparezcan cuando se añadan funciones con
  parámetros múltiples (`fn nombre { … }` con etiqueta de scope).
- **Etiquetas de `break`/`continue`** (futuro).

### Mensaje de error canónico

Categoría reservada — sin mensaje específico todavía. Cuando se
active, seguirá el patrón:

| Mensaje (plantilla)                                              | Cuándo                                              |
| ---------------------------------------------------------------- | --------------------------------------------------- |
| `unrecognized label '<etiqueta>' in <contexto>`                  | Una etiqueta obligatoria está mal escrita o ausente |
| `duplicate label '<etiqueta>'`                                   | Misma etiqueta aparece más de una vez en el ámbito   |
| `missing closing label '<etiqueta>'`                             | Etiqueta de apertura sin su par de cierre            |

---

## 5. Mensajes de error transversales (no clasificados arriba)

| Mensaje                                                     | Cuándo                                                        |
| ----------------------------------------------------------- | ------------------------------------------------------------- |
| `undefined identifier: 'x'`                                | Uso de `x` sin `nyan x` previo                                |
| `identifier 'x' used before its declaration`                | Primer uso de `x` antes de algún `nyan x` en orden textual     |
| `unused variable: 'x'`                                      | `nyan x = …` y `x` no aparece como sub-expresión de ningún statement posterior |

---

## 6. Forma de los reportes

Los mensajes de error semánticos siguen el mismo formato que los
errores léxicos y sintácticos para que el archivo de salida sea
homogéneo:

```
linea:columna -> mensaje
```

Por ejemplo:

```
3:5  -> type mismatch: expected int, found string
7:1  -> non-exhaustive match: default arm ('_') is required
9:12 -> undefined identifier: 'x'
```

El archivo de salida será `output/semantic_errors.txt`.

---

## 7. Lo que NO se incluye en esta primera iteración

- **Tipos construidos adicionales:** arrays, listas, mapas, tuplas,
  registros.
- **Conversiones implícitas** entre tipos (`int → string`, etc.).
- **Polimorfismo / sobrecarga de funciones.**
- **Genéricos.**
- **Funciones con parámetros** (todas las `fn neko` son nulas).
- **Recursión explícita entre funciones.**
- **`break`, `continue`, `return`** (estructuras de bucle no existen
  todavía).
- **Etiquetas de bloque** en general (§4 queda solo como categoría
  documentada por completud con la rúbrica de la materia).
- **Optimizaciones** de cualquier tipo.

Cualquier ampliación futura se documentará en una nueva sección al
final de este archivo, conservando intactas las reglas ya firmadas.
