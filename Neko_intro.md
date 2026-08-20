| LENGUAJES Y AUTOMATAS 1 |                                           |                                                                                               | ENERO – MAYO 2026               |
| ----------------------- | ----------------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------- |
|                         | INSTITUTO TECNOLÓGICO SUPERIOR DE URUAPAN | Ramses Neftali Chavez Sanchez Karol Ruben Quiroz Mora Profesora: Graciela Alicia Vizcaino Paz | NEKO Fecha: 13/05/2026 Página 1 |

## LENGUAJES Y AUTOMATAS 1

#### ENERO – MAYO 2026

# Introducción

Neko es un lenguaje de programación compilado, minimalista y orientado a expresiones, diseñado para ofrecer una sintaxis limpia, moderna y fácil de leer. Su enfoque principal es la simplicidad estructural y la reducción de código innecesario, aplicando un minimalismo agresivo que permite escribir instrucciones claras utilizando la menor cantidad de sintaxis posible. Neko incorpora características modernas inspiradas en lenguajes como Rust, Go y Kotlin, priorizando la expresividad mediante estructuras compactas y semánticamente estrictas. Entre las principales características del lenguaje destacan:

- Sintaxis minimalista y legible.
- Programación orientada a expresiones.
- _Pattern Matching_ simplificado.
- Inferencia estática de tipos.
- Evaluación de expresiones en estructuras condicionales.
- Variables inmutables.
- Alcance por bloques (_block scoping_).
- Retorno implícito de expresiones.
- Eliminación de sintaxis innecesaria como return y ;.
  El propósito principal de Neko es demostrar el funcionamiento interno de un compilador moderno, cubriendo las siguientes fases:

- Análisis léxico.
- Análisis sintáctico.
- Análisis semántico.
- Generación de código intermedio.
- Generación de código ensamblador.
  Neko busca demostrar cómo un lenguaje pequeño y estructurado puede mantener expresividad, claridad y coherencia semántica sin necesidad de estructuras complejas o verbosas. INSTITUTO TECNOLÓGICO SUPERIOR DE URUAPAN Página 2

# Alcance del Lenguaje

Neko será un lenguaje compilado orientado a la generación de código ensamblador (_Assembly_), enfocado en expresividad y simplicidad estructural. El lenguaje permitirá realizar las siguientes operaciones y características:

- Declaración de variables mediante la palabra reservada nyan.
- Inferencia estática de tipos.
- Variables inmutables.
- Operaciones aritméticas básicas.
- Comparaciones lógicas y relacionales.
- Evaluación de expresiones.
- Estructuras condicionales orientadas a expresiones.
- _Pattern Matching_ simplificado.
- Declaración de funciones sin parámetros.
- Impresión de resultados mediante meow.
- Comentarios de una sola línea utilizando #.
- Alcance local por bloques ({}).

#### Página 3

||LENGUAJES Y AUTOMATAS 1 ENERO – MAYO 2026|
|---|---|
||ALFABETO DE NEKO Letras:|
|A-Z a-z 0-9|Dígitos: Símbolos especiales:|
|+ - " \ _|* / % = < > ! & | ^ ~ () {} []; :,. ' ` @ $ ? Operadores compuestos:|
|&& || ++ -- |= |> :: ..|== != <= >= += -= *= /= -> => Espacios válidos: espacio tabulación salto de línea INSTITUTO TECNOLÓGICO SUPERIOR DE URUAPAN Página 4|

# TABLA LEXEMAS

|Componente Léxico|Patrón|Lexemas|
|---|---|---|
|Palabra reservada de inicio|neko|neko|
|Declaración de variables|nyan|nyan|
|Declaración de funciones|fn|fn|
|Condicional|if|if|
|Alternativa condicional|else|else|
|Pattern Matching|match|match|
|Caso por defecto|_|_|
|Impresión en consola|meow|meow|
|Nombres de variables|(a-z A-Z)(a-z A-Z 0-9 _ )*|cat, kitty, furball, nekoVar, whiskers1|
|Números enteros|(0-9)+|0, 7, 15, 999|
|Cadenas de texto|"((a-z) (A-Z) (0-9) ( ) (_) )*"|"Hello", "Neko", "Cat123"|
|Operador de asignación|=|=|
|Cadenas de texto|"((a-z) | (A-Z) | (0-9) | ( ) | (_) )*"|"Hello", "Neko", "Cat123"|
|Operador de asignación|=|=|
|Operador suma|+|+|
|Operador resta|-|-|
|Operador multiplicación|*|*|
|Operador división|/|/|
|Operador módulo|%|%|
|Operador igual comparación|==|==|
|Operador diferente|!=|!=|
|Operador mayor que|>|>|
|Operador menor que|<|<|
|Operador mayor o igual|>=|>=|
|Operador menor o igual|<=|<=|
|Operador lógico AND|&&|&&|
|Operador lógico OR|||||||
|Operador lógico NOT|!|!|
|Operador de expresión|=>|=>|
|Paréntesis izquierdo|(|(|
|Paréntesis derecho|)|)|
|Llave izquierda|{|{|
|Llave derecha|}|}|
|Separador|,|,|
|Comentarios|#((a-z) (A-Z) (0-9) ( ) (_) )*|# comentario, # Neko Language, # variable_1|
|Booleanos|true false|true, false|

#### Página 6

### Restricciones Semánticas del Lenguaje

Con el objetivo de mantener coherencia semántica y simplicidad de compilación, Neko establece las siguientes reglas:

#### Inferencia estática de tipos

Las variables no requieren especificar explícitamente su tipo, ya que este es inferido automáticamente por el compilador.

Ejemplo:

```neko
nyan score = 90
```

Una vez inferido el tipo, este no puede cambiar posteriormente.

#### Inmutabilidad de variables

Las variables declaradas en Neko son inmutables.

Ejemplo inválido:

```neko
nyan score = 90
score = 100
```

#### Scope por bloques

Las variables únicamente existen dentro del bloque donde fueron declaradas.

Ejemplo:

```neko
if true {
nyan value = 10
}
meow(value) # error semántico
```

#### Página 7

### No redeclaración de variables

No está permitido redeclarar una variable dentro del mismo bloque o alcance.

Ejemplo inválido:

```neko
nyan cat = 10
nyan cat = 20
```

#### Evaluación orientada a expresiones

Las estructuras condicionales y match producen un valor.

Ejemplo:

```neko
nyan result = if score > 70{
    "Pass"
} else {
    "Fail"
}
```

#### Compatibilidad en tipos de expresiones

Las expresiones deben retornar el mismo tipo en todas sus ramas.

Ejemplo válido:

```neko
nyan value = if true{
    10
} else {
    20
}
```

Ejemplo inválido:

```neko
nyan value = if true{
    10
} else {
    "hello"
}
```

#### Página 8

### Retorno implícito

Las funciones no utilizan la palabra reservada return.

El valor de retorno corresponde automáticamente a la última expresión evaluada dentro del bloque.

Ejemplo:

```neko
fn neko {
    "Neko Language"
}
```

#### Eliminación de terminadores de instrucción

Neko no utiliza ; como terminador de instrucciones.
Las instrucciones finalizan mediante:
• salto de línea  
• cierre de bloque con {}

#### Página 9

### Filosofía de Diseño

Neko sigue la filosofía:

"Menos sintaxis, mayor expresividad."

El lenguaje busca eliminar estructuras innecesarias y reducir el ruido visual del código, promoviendo un estilo compacto, moderno y legible.

### Su diseño prioriza:

- claridad visual,
- simplicidad estructural,
- coherencia semántica,
- y expresividad mediante evaluación de expresiones.

Neko adopta un enfoque minimalista donde pequeñas construcciones sintácticas permiten representar lógica compleja de manera sencilla y elegante.

#### Página 10

### Ejemplos de Programas

#### Ejemplo 1 — Pattern Matching

```neko
# Evaluación de score utilizando pattern matching

fn neko{
    nyan score = 90

    nyan result = match score {
        100 => "Perfect"
        90 => "Excellent"
        70 => "Pass"
        _ => "Fail"
    }
    meow(result)
}
```

#### Ejemplo 2 — Expresiones condicionales

```neko
# Expresiones orientadas a evaluación

fn neko{

    nyan a = 10
    nyan b = 5

    nyan result = if a > b {
        a + b
    } else{
        a - b
    }
    meow(result)
}
```

#### Ejemplo 3 — Scope por bloques

```neko
# Scope local por bloques
fn neko {
    if true {
        nyan cat = "Neko"
        meow(cat)
    }
}
```

#### Ejemplo 4 — Retorno implícito

```neko
# Retorno implícito de expresiones
fn neko {
 "Neko Compiler"
}
```
