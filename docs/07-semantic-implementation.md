# Neko — Implementación del Análisis Semántico

> Documento **07** de la serie [`docs/`](./00-index.md).
> Describe cómo se implementaron en Rust las cuatro comprobaciones
> estáticas comprometidas en [`06-semantic-conventions.md`](./06-semantic-conventions.md).
> Las convenciones dicen **qué** se valida; este documento dice
> **cómo** se valida.

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
   └────┬─────┘
        │ Program (AST)
        ▼
   ┌──────────────┐
   │   semantic   │  ──►  output/semantic_errors.txt
   └──────────────┘
        │ SemanticAnalysis
        ▼
   (codegen en iteraciones futuras)
```

El análisis semántico **solo corre** si las fases léxica y sintáctica
produjeron cero errores. Esto evita cascadas de errores cuando los
identificadores no se reconocieron o los nodos del AST son placeholders
de recuperación.

---

## 2. Estructura de `src/semantic/`

```
src/semantic/
├── mod.rs           ← declaración de submódulos + re-exports
├── types.rs         ← enum Type { Int, String, Bool, Fn }
├── scope.rs         ← Scope (parent-linked) + Symbol
├── error.rs         ← SemanticError
├── analyzer.rs      ← SemanticAnalyzer (visitor sobre el AST)
└── tests.rs         ← 16 unit tests (uno por categoría de comprobación)
```

Cada archivo lleva un comentario de cabecera que explica **por qué**
existe y **cómo** se relaciona con la categoría correspondiente de
[`06-semantic-conventions.md`](./06-semantic-conventions.md).

---

## 3. El problema de las posiciones en el AST y cómo se resolvió

El AST producido por el parser **no lleva** información de línea y
columna en cada nodo. Sin esa información no podemos reportar errores
semánticos con la precisión `linea:col -> mensaje` que usa el lexer y
el parser.

**Solución adoptada:** el analizador semántico recibe, además del
`Program`, el `Vec<Token>` original. Mantiene un cursor (`self.cursor`)
sobre los tokens y avanza en lockstep con la visita del AST. Cada
`visit_*` consume los tokens que el AST le dice que están ahí, y
cuando necesita reportar un error lee `self.current_pos()` (o
`last_consumed_position()` para errores post-consumo).

Esto funciona porque el parser y el lexer están sincronizados: el
parser emite exactamente los mismos tokens que produjo el lexer, en el
mismo orden. Por eso un `Stmt::VarDecl { name, value }` siempre
corresponde a los tokens `nyan`, `<id>`, `=`, `<expr tokens…>`.

Si en una iteración futura el AST se anota con posiciones explícitas
(por ejemplo, envolviendo cada nodo en un `Spanned<T>` con
`{line, column}`), se puede eliminar el cursor sobre tokens. Para
esta primera iteración del proyecto educativo la solución del cursor
es más simple y evita tocar el parser.

---

## 4. Comprobación 1 — Tipos

**Cómo se implementa:**

- Cada nodo AST tiene una función `visit_*` que devuelve un `Type`:
  el tipo inferido de esa sub-expresión.
- Los tipos básicos se infieren trivialmente en `visit_primary`:
  `Literal::Int → Type::Int`, etc.
- Para operadores binarios (`+`, `-`, `&&`, etc.) el visitor comprueba
  que los operandos tengan el tipo esperado y devuelve el tipo
  resultado (que suele ser el mismo que el de los operandos, excepto
  para comparaciones que siempre devuelven `bool`).
- El tipo de un identificador (`Literal::Ident(name)`) se resuelve
  consultando la `Scope`. Si no se encuentra, se emite "undefined
  identifier" y se devuelve `Type::Int` (placeholder) para que el
  resto del análisis continúe sin cascadas.

**Casos especiales documentados:**

- **Comparaciones:** ambos operandos deben tener el mismo tipo (no se
  permite comparar `int` con `string`).
- **`if` con `else`:** ambos bloques deben devolver el mismo tipo.
  Sin `else` no se impone restricción.
- **`match`:** todos los brazos deben devolver el mismo tipo entre sí.

---

## 5. Comprobación 2 — Flujo de control

**Cómo se implementa:**

- La única transferencia de control "compleja" de Neko en esta
  iteración es `match`. El visitor de `match` lleva un flag
  `has_default` que se activa cuando aparece un brazo con patrón
  `Wildcard`. Si al cerrar el match el flag sigue `false`, emite
  `non-exhaustive match: default arm ('_') is required`.
- Adicionalmente se detectan **patrones duplicados** dentro del mismo
  match (otro caso de unicidad + flujo de control).

---

## 6. Comprobación 3 — Unicidad

**Cómo se implementa:**

- **Sin redeclaración:** al visitar `nyan x = …`, se llama a
  `scope.contains_local("x")`. Si devuelve `true`, se emite
  `redeclaration of 'x' in the same scope`.
- **Sin shadowing:** además se llama a `scope.contains_in_chain("x")`
  que recorre la cadena de scopes padre. Si encuentra `x` allí pero
  no en el scope actual, emite `'x' shadows an outer declaration
  (Neko disallows shadowing)`.
- **Sin brazos duplicados en `match`:** durante la visita de los
  brazos se acumulan los patrones literales en `seen_patterns` y se
  compara cada nuevo brazo contra esa lista.

**Por qué `Scope` necesita dos métodos:**

- `contains_local(name)`: solo el scope actual. Distingue "redeclaración
  en el mismo ámbito" (error claro: ya existe aquí mismo).
- `contains_in_chain(name)`: scope actual + padres. Detecta shadowing
  (existe arriba, no aquí). La combinación de las dos respuestas
  determina qué mensaje emitir.

---

## 7. Comprobaciones relacionadas con nombres

**Estado:** reservado, sin implementación activa. La categoría se
documenta en [`06-semantic-conventions.md`](./06-semantic-conventions.md)
§4 para completud con la rúbrica de la materia, pero Neko en esta
iteración no tiene bloques con etiqueta explícita. Cuando se añadan
(por ejemplo, cuando se introduzcan funciones con parámetros múltiples
o plantillas tipo HTML), esta sección se ampliará con un visitor
dedicado.

---

## 8. Comprobaciones auxiliares

### 8.1 Identificador indefinido / uso antes de declaración

Ambas se manifiestan igual: en `visit_ident(name)`, si
`scope.lookup(name)` devuelve `None`, se emite `undefined identifier:
'name'`. Esto cubre **automáticamente** el caso "uso antes de
declaración", porque el visitor recorre las statements en orden textual:
si `nyan x` no ha aparecido todavía, `x` no está en el scope.

### 8.2 Variable declarada pero no usada

Se detecta en una **segunda pasada** (`analyze`, después de la visita):

```rust
for d in &self.declared {
    let used = self.read_count.get(&d.name).copied().unwrap_or(0);
    if used == 0 {
        // emit "unused variable: 'x'"
    }
}
```

`read_count` se incrementa cada vez que `visit_ident` resuelve un
nombre en el scope. Si al final del análisis el contador es 0, la
variable nunca se leyó.

---

## 9. ¿Por qué dos pasadas en lugar de una?

El análisis semántico recorre el programa una sola vez para los
visit, y luego hace una segunda pasada ligera para detectar variables
no usadas. La alternativa (detectar uso-no-usada en línea) requeriría
**dos pasadas de todos modos** (una para encontrar las declaraciones,
otra para encontrar los usos), y complicaría el visitor. Mantener la
lógica separada por pasada hace el código más legible y testeable.

---

## 10. Forma del reporte

Igual que las dos fases anteriores:

```
linea:col -> mensaje
```

Archivo: `output/semantic_errors.txt`.

El resumen en terminal indica los conteos de las tres fases:

```
OK — 44 tokens, 4 symbols, parse tree built with 0 syntactic errors,
     semantic analysis: 0 errors.

# o, si hay errores:

Errors found — lexical: 0, syntactic: 0, semantic: 3. See output/errors.txt,
output/parse_errors.txt, output/semantic_errors.txt.
```

---

## 11. Limitaciones conocidas

- **El bloque de `if`/`else` y de brazos de `match` sigue siendo de
  una sola expresión.** Esto es por la decisión de diseño
  documentada en [`03-grammar-design.md`](./03-grammar-design.md) §2.5
  y [`05-parser.md`](./05-parser.md) §6. Si en una iteración futura
  se amplía a multi-statement, las pruebas del visitor para
  shadowing anidado se vuelven más naturales.

- **Las posiciones reportadas para errores de "unused variable"
  corresponden al último token consumido, no a la línea exacta de
  declaración.** Para un proyecto educativo es suficiente; un
  compilador de producción querría anotar cada `Stmt::VarDecl` con
  la posición de `nyan`.

- **`Type::Fn` está definido pero no se construye.** Se reserva para
  cuando se agreguen funciones con parámetros, donde aparecerá un
  `Type::Fn { params: Vec<Type>, ret: Box<Type> }` (futuro).

---

## 12. Cobertura de tests

16 pruebas unitarias en `src/semantic/tests.rs`, agrupadas por
categoría:

| Categoría                              | Tests                                                                              |
| -------------------------------------- | ---------------------------------------------------------------------------------- |
| **Tipos** (comprobación 1)             | `type_check_int_addition_ok`, `type_check_string_plus_int_is_error`, `type_check_if_must_be_bool`, `type_check_branches_must_match`, `type_check_negate_string_is_error`, `type_check_logical_operands_must_be_bool` |
| **Flujo de control** (comprobación 2)  | `match_must_have_default_arm`, `match_with_default_arm_is_ok`                       |
| **Unicidad** (comprobación 3)          | `redeclaration_in_same_scope_errors`, `shadowing_inner_block_errors`, `duplicate_match_arm_pattern_errors` |
| **Auxiliares**                         | `undefined_identifier_errors`, `use_before_declaration_errors`, `unused_variable_errors`, `used_variable_does_not_trigger_unused`, `empty_program_is_clean` |

Más las 8 pruebas de parser que ya existían, total **24 pruebas
pasando**:

```
$ cargo test
test result: ok. 24 passed; 0 failed
```
