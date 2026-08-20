**This is my class' (and teacher's) grammar example**

## Análisis sintáctico

**Tareas del analizador sintáctico**:

1. Cada estructura utilice todas los componentes léxicos que requiere de acuerdo a lo especificado en la gramática del lenguaje
2. Que todos los componentes léxicos que utiliza estén ubicados en el orden correcto

**¿Qué debe hacer el analizador sintáctico de mi proyecto?**

- Tomar como entrada un código LIBRE DE ERRORES LÉXICOS y verificar con base en su gramática, que se cumplan las dos condiciones implícitas en su tarea. Esto es:
  1.  Cada instrucción debe tener todo lo que requiere
  2.  Cada elemento debe estar en el orden correcto

#### Ejemplo de programa

```alicia
ini{
ent x;
ent y;
ent y2;
ent d1;
ent sum;

>> "Introduce los valores x, y y y2";

x << ;
y << ;
y2 << ;
sum = x + y + y2 ;
div = (x + y) / y2;
>> "Resultado de la suma" , sum;
>> >> "Resultado de la división" , d1;
}
```

#### Ejemplo de gramática

```alicia-gramatica
Prog → ini { De Ln }

Dc → ent var ; Dc | dec var ; Dc | ε

In → Fn In | Op In | ε

Fn → >> msg ; | var << ; | >> msg , Dato;

Op → var = Dato ; | var = Exp ;

Exp → Dato + Exp | Dato - Exp | Dato * Exp | Dato ÷ Exp
    | Dato | (Exp) | Exp + Exp | Exp - Exp | Exp * Exp
    | Exp ÷ Exp

Dato → var | ve | vd`
```
