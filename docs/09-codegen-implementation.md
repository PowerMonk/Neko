# Neko — Generacion de Codigo Ensamblador

> Documento **09** de la serie [`docs/`](./00-index.md).
> Describe la fase final del compilador: como se traduce el codigo
> intermedio (cuadruplos) a un programa en ensamblador x86-64 que corre
> en Linux.
>
> Documento complementario: [`08-icg-implementation.md`](./08-icg-implementation.md)
> describe la fase anterior (los cuadruplos y triplos).

---

## 1. Posicion en el pipeline

```
   .neko source
        |
        v
   +-----------+
   |  lexer    | --> output/tokens.txt, output/symbols.txt, output/errors.txt
   +-----+-----+
         | Vec<Token>
         v
   +-----------+
   |  parser   | --> output/parse_tree.txt, output/parse_errors.txt
   +-----+-----+
         | Program (AST)
         v
   +-------------+
   |  semantic   | --> output/semantic_errors.txt
   +------+------+
          | Program (AST) + tabla de simbolos con tipos
          v
   +----------+
   |   icg    | --> output/quadruples.txt, output/triples.txt
   +----+-----+
        | Vec<Quadruple> + tabla de simbolos
        v
   +----------+
   | codegen  | --> output/program.asm, output/symbol_table.txt
   +----+-----+
        |
        v
   nasm -> ld -> ./program     (la salida del programa real)
```

La fase de codegen **solo corre** si las cuatro fases anteriores
produjeron cero errores.

---

## 2. Objetivo

| Aspecto | Decision |
|---|---|
| Arquitectura | x86-64 |
| Sistema operativo | Linux (syscalls directas, sin libc) |
| Sintaxis | NASM |
| Punto de entrada | `_start` (convención de Linux, no `main`) |
| Formato del objeto | ELF64 (`nasm -f elf64`) |
| Enlazado | `ld` directo, sin `libc` |

**Por que `syscall` y no `int 0x80`:** `int 0x80` es la interfaz de
32 bits heredada; en modo 64 bits `syscall` con `rax`/`rdi`/`rsi`/`rdx`
es la interfaz correcta y la que usa el kernel Linux actual.

---

## 3. Estructura de `src/codegen/`

```
src/codegen/
├── mod.rs            <- declaraciones + re-exports
├── symbol_table.rs   <- SymbolTable: nombre -> direccion (seccion + offset)
├── runtime.rs        <- rutinas del runtime, como plantillas de texto
├── assembler.rs      <- Assembler: Vec<Quadruple> -> texto NASM
├── runner.rs         <- ensambla, enlaza y ejecuta el .asm via WSL
└── tests.rs          <- 34 pruebas unitarias
```

---

## 4. Modelo de memoria

Tres secciones, igual que cualquier programa en ensamblador:

### 4.1 `section .data` — datos inicializados

Contiene:

- Los **literales de cadena** del programa, cada uno terminado en NUL:
  ```asm
  str_0:         db "Hola mundo desde Neko", 0
  ```
- Los textos del runtime:
  ```asm
  str_true:      db "true", 0
  str_false:     db "false", 0
  ```

El terminador NUL es essential: `neko_print_str` calcula la longitud
recorriendo la cadena hasta encontrarlo.

### 4.2 `section .bss` — datos no inicializados

El sistema operativo pone esta seccion en cero al cargar el programa.
Contiene:

- Los **simbolos del runtime**, precargados en la tabla de simbolos:

  | Simbolo | Tamano | Proposito |
  |---------|--------|-----------|
  | `stdin` | 256 bytes | **Bufer de entrada del teclado** |
  | `stdin_len` | 8 bytes | Bytes leidos en la ultima lectura |
  | `stdout` | 8 bytes | Descriptor de salida (fd 1) |
  | `numbuf` | 256 bytes | Buffer auxiliar para convertir int a texto |

  > El bucle de `stdin` es la "entrada del usuario" que mencionan las
  > notas de la maestra: la tabla de simbolos lo contiene desde el
  > principio, y la rutina `neko_read_line` deposita ahi lo que el
  > usuario teclee.

- Las **variables del usuario** (una ranura de 8 bytes cada una).
- Los **temporales del compilador** `T1`, `T2`, … (8 bytes cada uno).

### 4.3 `section .text` — el codigo

El programa propiamente dicho, mas las rutinas del runtime.

### 4.4 El stack

El stack **no** aparece en la tabla de simbolos. Se reserva una sola
vez, al inicio del programa:

```asm
STACK_SIZE equ 4096

_start:
    mov  rbp, rsp
    sub  rsp, STACK_SIZE
```

Neko tiene una unica funcion (`fn neko`) sin parametros, asi que no hay
frames que gestionar: todo el programa corre en un unico frame de stack.
El tamano es fijo y se documenta con la constante `STACK_SIZE`.

> Cuando se agreguen funciones con parametros, habra que revisar esta
> decision: los frames passarian a reservarse en el stack con offsets
> desde `rbp`, y los temporales locales dejarian de estar en `.bss`.

---

## 5. La tabla de simbolos

### 5.1 Estructura

```rust
pub struct Symbol {
    pub name: String,
    pub section: Section,   // .data o .bss
    pub offset: usize,      // byte offset dentro de la seccion = "direccion"
    pub size: usize,
    pub unit: Unit,         // Bytes o Quadwords (define la directiva)
    pub kind: SymbolKind,   // Variable | Temporary | StringLiteral | Runtime
    pub ty: Option<Type>,   // el tipo estatico, si se conoce
}
```

El `offset` es la **direccion relocatable** del simbolo: la posicion
dentro de su seccion. Es el mismo modelo que usan los ensambladores y
enlazadores reales; el `LABEL` que emitimos en el `.asm` es lo que
finalmente el enlazador traduce a una direccion fisica.

### 5.2 Las dos tablas, y por que hacen falta las dos

Este es el punto central de la fase, y el que hay que explicar en la
defensa:

```
Tabla de SIMBOLOS   ->  DONDE vive cada variable (seccion, offset, tamano, tipo)
Tabla de CUADRUPLOS ->  QUE operaciones hace el programa, y en que orden
```

Los cuadruplos **noman** las cosas: "suma `a` y `b` en `T1`". No dicen
donde viven `a`, `b` ni `T1`. Eso lo responde la tabla de simbolos.

Por eso el generador de codigo necesita **las dos** tablas:

- Los **cuadruplos** vienen de la fase ICG.
- La **tabla de simbolos con tipos** viene de la fase semantica
  (`SemanticAnalysis::symbols`), porque el cuadruplo no lleva el tipo:
  cuando el generador traduce un `PRINT`, necesita saber si el operando
  es `int`, `string` o `bool` para elegir la rutina correcta.

### 5.3 Tipos de los temporales

Los temporales no estan en la tabla de simbolos semantica (el compilador
los crea), asi que el generador rastrea su tipo mientras traduce:

| Cuadruplo que produce el temporal | Tipo del temporal |
|-----------------------------------|-------------------|
| `(+, -, *, /, %, -)`              | `int`             |
| `(==, !=, <, >, <=, >=)`          | `bool`            |
| `(&&, \|\|)`                     | `bool`            |
| `(!)`                            | `bool`            |
| `(=, <origen>, _, T)`             | el tipo del origen |

---

## 6. Traduccion de cuadruplos a instrucciones

Cada cuadruplo se convierte en una secuencia fija de instrucciones:

| Cuadruplo | Instrucciones generadas |
|---|---|
| `(+, a, b, T)` | `mov rax, a` / `mov rcx, b` / `add rax, rcx` / `mov [T], rax` |
| `(-, a, b, T)` | idem con `sub rax, rcx` |
| `(*, a, b, T)` | idem con `imul rax, rcx` |
| `(/, a, b, T)` | `mov rax, a` / `mov rcx, b` / `cqo` / `idiv rcx` / `mov [T], rax` |
| `(%, a, b, T)` | idem pero `mov [T], rdx` (el resto queda en `rdx`) |
| `(==, a, b, T)` | `cmp rax, rcx` / `sete al` / `movzx rax, al` / `mov [T], rax` |
| `(!=, a, b, T)` | idem con `setne al` |
| `(<, a, b, T)` | idem con `setl al` |
| `(>, a, b, T)` | idem con `setg al` |
| `(<=, a, b, T)` | idem con `setle al` |
| `(>=, a, b, T)` | idem con `setge al` |
| `(&&, a, b, T)` | `mov rax, a` / `mov rcx, b` / `and rax, rcx` / `mov [T], rax` |
| `(\|\|, a, b, T)` | idem con `or rax, rcx` |
| `(!, a, _, T)` | `mov rax, a` / `xor rax, 1` / `mov [T], rax` |
| `(-, a, _, T)` | `mov rax, a` / `neg rax` / `mov [T], rax` (unario) |
| `(=, src, _, d)` | `mov rax, src` / `mov [d], rax` |
| `(PRINT, x, _, _)` | `mov rdi, x` / `call neko_print_{str,int,bool}` |
| `(GOTO, _, _, L)` | `jmp L` |
| `(GOTOF, c, _, L)` | `mov rax, c` / `test rax, rax` / `jz L` |
| `(GOTOV, c, _, L)` | `mov rax, c` / `test rax, rax` / `jnz L` |
| `(LABEL, _, _, L)` | `L:` |

### 6.1 Clasificacion de operandos

El generador decide como cargar cada operando:

| Operando | Como se reconoce | Carga |
|---|---|---|
| Literal entero | Solo digitos | `mov rax, 42` |
| Literal string | Empieza y acaba con `"` | `lea rax, [rel str_x]` |
| Literal bool | `true` / `false` | `mov rax, 0` / `mov rax, 1` |
| Variable | Nombre de la tabla semantica | `mov rax, [var_x]` |
| Temporal | `T` + digitos | `mov rax, [T1]` |
| Etiqueta | `L` + digitos | Destino de `jmp` / `jz` / `jnz` |
| `_` | Slot sin uso | `xor rax, rax` |

### 6.2 Las cadenas se representan por su direccion

Un valor de tipo `string` en Neko **es la direccion** de su texto. Por
eso:

- `(=, "Hola", _, x)` → `lea rax, [rel str_0]` + `mov [var_x], rax`
  (guarda la direccion del literal en la variable).
- `(PRINT, x, _, _)` → `mov rdi, [var_x]` + `call neko_print_str`
  (recupera la direccion y la pasa a la rutina).

### 6.3 El menos unario frente al binario

Los dos usan el operador `-`. Se distinguen porque el unario tiene
`_` en el segundo argumento:

```asm
; binario:  ( -, a, b, T )
    mov rax, [var_a]
    mov rcx, [var_b]
    sub rax, rcx
    mov [T1], rax

; unario:   ( -, a, _, T )
    mov rax, [var_a]
    neg rax
    mov [T1], rax
```

> **Cuidado:** en el generador, el caso del menos unario debe
> aparecer **antes** del caso de la aritmetica binaria. Rust evalua los
> `match` en orden, asi que si el binario va primero, el unario queda
> inalcanzable y `neg` nunca se genera. (El compilador de Rust avisa
> de esto con `unreachable_patterns`.)

---

## 7. Las rutinas del runtime

Van **dentro** del `.asm` generado, de modo que el archivo es
autocontenido: se ensambla y ejecuta sin nada mas.

### 7.1 Las interrupciones de entrada y salida

Las notas de la maestra mencionan "print on the screen the in and out
interruptions". Son las llamadas al sistema de Linux:

| Interrupcion | `rax` | Argumentos | Uso |
|---|---|---|---|
| `read`  | 0 | `rdi`=fd, `rsi`=buffer, `rdx`=count | **Entrada**: fd 0 = teclado |
| `write` | 1 | `rdi`=fd, `rsi`=buffer, `rdx`=count | **Salida**: fd 1 = pantalla |
| `exit`  | 60 | `rdi`=codigo | Terminar el programa |

### 7.2 `neko_print_str`

```
in:  rdi = puntero a una cadena terminada en NUL
```

Calcula la longitud recorriendo hasta el NUL y hace un `write`.

### 7.3 `neko_print_int`

```
in:  rdi = el valor (entero con signo de 64 bits)
```

Construye los digitos decimales **hacia atras** en `numbuf` (desde
`numbuf_end` hacia el inicio) y despues los escribe. El signo se
guarda en `r8` porque `div` destruye `rdx`.

### 7.4 `neko_print_bool`

```
in:  rdi = 0 (false) o distinto de 0 (true)
```

Imprime `true` o `false` usando las etiquetas `str_true` / `str_false`.

### 7.5 `neko_read_line`

```
in:  rdi = buffer destino, rsi = maximo de bytes
out: rax = bytes leidos
```

Hace `read(0, buffer, max)`. Es la rutina que usaria el futuro
`read()` del lenguaje; en esta iteracion Neko todavia no tiene una
instruccion de lectura, pero la infraestructura esta lista y el bufer
`stdin` esta en la tabla de simbolos.

### 7.6 Convencion de llamada

Las rutinas se invocan con `call`, asi que respetan la ABI de System V
AMD64:

- `rsp` debe estar alineado a 16 bytes en el sitio del `call`.
- Los argumentos van en `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9`.
- El valor de retorno va en `rax`.
- `rbx`, `rbp`, `r12`–`r15` se preservan.

Dentro de una rutina:

```
    push rbp          ; rsp pasa a estar ≡ 0 (mod 16)
    mov  rbp, rsp
    sub  rsp, 16      ; sigue en ≡ 0 (mod 16)  ✓ alineado
    ...
    add  rsp, 16
    pop  rbp
    ret
```

La instruccion `syscall` en si no exige alineacion (esa regla es para
`call`), pero el frame se mantiene alineado para poder invocar estas
rutinas desde cualquier parte.

---

## 8. Estructura del archivo generado

```asm
; ================================================================
; Neko — programa compilado
; Generado por neko-compiler
;
; Objetivo : Linux x86-64, sintaxis NASM
; Ensamblar: nasm -f elf64 program.asm -o program.o
; Enlazar  : ld program.o -o program
; Ejecutar : ./program
; ================================================================

section .data

    ; --- textos del runtime ---
    str_true:      db "true", 0
    str_false:     db "false", 0

    ; --- literales de cadena del programa ---
    str_0:         db "Hola mundo desde Neko", 0

section .bss

    ; --- runtime: entrada del teclado y búferes ---
    stdin:         resb 256
    stdin_len:     resq 1
    stdout:        resq 1
    numbuf:        resb 256
    numbuf_end:    equ numbuf + 256

    ; --- variables del usuario ---
    var_score:     resq 1

    ; --- temporales del compilador ---
    T1:            resq 1

section .text
global _start

STACK_SIZE equ 4096

_start:
    mov  rbp, rsp
    sub  rsp, 4096

    ; ---- cuerpo del programa (generado desde los cuadruplos) ----
    mov  rax, 90
    mov  [var_score], rax
    lea  rax, [rel str_0]
    mov  [var_result], rax
    mov  rdi, [var_result]
    call neko_print_str

    ; ---- salida: exit(0) ----
    mov  rax, 60
    xor  rdi, rdi
    syscall

; ================================================================
; Rutinas del runtime de Neko
; Las interrupciones de entrada y salida del sistema:
;   read  (rax=0)  -> fd 0 = teclado
;   write (rax=1)  -> fd 1 = pantalla
;   exit  (rax=60) -> fin del programa
; ================================================================
; ... rutinas ...
```

**Nota sobre `resb` vs `resq`:** la directiva debe corresponder a la
unidad del simbolo. `resb 256` reserva 256 **bytes**; `resq 256`
reservaria 256 quadwords (2048 bytes). Por eso `Symbol` lleva un campo
`unit` (`Bytes` o `Quadwords`) que decide la directiva.

**Nota sobre los `LABEL`:** las variables del usuario se emiten con el
prefijo `var_` (`var_score`) para no colisionar con las etiquetas del
runtime y con los nombres de los temporales.

---

## 9. Ejecucion automatica

El compilador, tras generar el `.asm`, lo ensambla, lo enlaza y lo
ejecuta. En Windows usa **WSL**:

```
wsl -- bash -c "cd /mnt/c/ruta/al/proyecto/output && \
                nasm -f elf64 program.asm -o program.o && \
                ld program.o -o program && \
                chmod +x program && \
                ./program"
```

La traduccion de rutas la hace `runner::to_wsl_path`:

| Ruta de Windows | Ruta en WSL |
|---|---|
| `C:\Users\aroml\Neko\output` | `/mnt/c/Users/aroml/Neko/output` |
| `D:\work\out` | `/mnt/d/work/out` |
| `/home/user/neko` | `/home/user/neko` (sin cambios) |

Si WSL no esta disponible, el compilador **no falla**: imprime las
instrucciones manuales para ejecutarlo en cualquier Linux.

El flag `--no-run` genera el `.asm` sin ensamblarlo ni ejecutarlo.

---

## 10. Cobertura de tests

34 pruebas unitarias en `src/codegen/tests.rs`:

| Categoria              | Tests                                                                                |
| ---------------------- | ------------------------------------------------------------------------------------ |
| **Tabla de simbolos**  | `runtime_symbols_are_preloaded`, `variables_and_temporaries_get_distinct_offsets`, `identical_string_literals_share_one_symbol`, `variable_labels_are_prefixed` |
| **Traduccion de ops**  | `addition_uses_add_instruction`, `subtraction_uses_sub_instruction`, `multiplication_uses_imul`, `division_uses_signed_division_and_keeps_quotient`, `modulo_keeps_the_remainder`, `unary_minus_uses_neg_not_sub` |
| **Comparaciones**      | `comparisons_use_setcc_and_movzx` (verifica los 6 operadores)                        |
| **Logica**             | `logical_and_uses_and_instruction`, `logical_or_uses_or_instruction`, `logical_not_uses_xor_with_one` |
| **Asignacion**         | `assignment_stores_into_the_destination`                                            |
| **Literales**          | `integer_literal_is_moved_immediately`, `string_literal_is_loaded_by_address`        |
| **Impresion**          | `print_int_uses_print_int_routine`, `print_string_uses_print_str_routine`, `print_bool_uses_print_bool_routine`, `print_literal_string_uses_print_str_routine` |
| **Control de flujo**   | `goto_becomes_jmp`, `gotof_becomes_test_and_jz`, `gotov_becomes_test_and_jnz`, `label_becomes_a_label` |
| **Estructura del archivo** | `file_has_the_three_sections`, `entry_point_is_global_and_exits_cleanly`, `stack_is_reserved_with_the_declared_size`, `runtime_routines_are_embedded`, `runtime_uses_the_linux_syscalls`, `string_literals_are_nul_terminated`, `runtime_texts_are_emitted`, `keyboard_buffer_is_in_the_bss_section` |
| **Rutas de WSL**       | `windows_path_becomes_a_mnt_path`, `linux_path_is_left_alone`, `drive_letter_is_lowercased` |

Total del proyecto: **78 pruebas pasando** (8 de parser, 16 de
semantica, 20 de ICG, 34 de codegen).

```
$ cargo test
test result: ok. 78 passed; 0 failed
```

---

## 11. Limitaciones conocidas

- **`&&` y `||` no cortocircuitan.** Ambos operandos se evaluan
  siempre. Es una simplificacion consciente de un compilador
  educativo: el cortocircuito exigiria ramas de codigo por cada
  operador logico.
- **Sin optimizacion.** No hay constant folding ni propagacion de
  copias. Cada operacion genera un temporal.
- **Cadenas solo de lectura en el literal.** El codigo generado no
  puede modificar una cadena en memoria (no hay sintaxis para ello en
  Neko).
- **Un solo frame de stack.** Valido mientras Neko tenga una unica
  funcion sin parametros.
- **Sin division por cero en tiempo de compilacion.** `5 / 0` se
  compila y falla en ejecucion (es una comprobacion dinamica, no
  estatica).
- **El `.asm` es especifico de Linux x86-64.** Para correrlo en
  Windows habria que generar una variante con la API de Win32.
