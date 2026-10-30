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
    str_1:         db "Perfect", 0
    str_2:         db "Excellent", 0
    str_3:         db "Fail", 0

section .bss

    ; --- runtime: entrada del teclado y búferes ---
    stdin:         resb 256
    stdin_len:     resq 1
    stdout:        resq 1
    numbuf:        resb 256
    numbuf_end:    equ numbuf + 256

    ; --- variables del usuario ---
    var_score:     resq 1
    var_banana:    resq 1
    var_holamundo: resq 1
    var_result:    resq 1

    ; --- temporales del compilador ---
    T1:            resq 1
    T2:            resq 1
    T3:            resq 1

section .text
global _start

STACK_SIZE equ 4096

_start:
    ; Reserva del stack. Neko tiene una sola funcion
    ; (fn neko) sin parametros, asi que no hay frames:
    ; todo el programa corre en un unico frame.
    mov  rbp, rsp
    sub  rsp, 4096

    ; ---- cuerpo del programa (generado desde los cuadruplos) ----
    mov rax, 90
    mov [var_score], rax
    mov rax, 90
    mov [var_banana], rax
    lea rax, [rel str_0]
    mov [var_holamundo], rax
    mov rax, [var_score]
    mov rcx, 100
    cmp rax, rcx
    sete al
    movzx rax, al
    mov [T2], rax
    mov rax, [T2]
    test rax, rax
    jnz L2
    mov rax, [var_score]
    mov rcx, 90
    cmp rax, rcx
    sete al
    movzx rax, al
    mov [T3], rax
    mov rax, [T3]
    test rax, rax
    jnz L3
    jmp L4
L2:
    lea rax, [rel str_1]
    mov [T1], rax
    jmp L1
L3:
    lea rax, [rel str_2]
    mov [T1], rax
    jmp L1
L4:
    lea rax, [rel str_3]
    mov [T1], rax
L1:
    mov rax, [T1]
    mov [var_result], rax
    mov rdi, [var_result]
    call neko_print_str
    mov rdi, [var_holamundo]
    call neko_print_str
    mov rdi, [var_banana]
    call neko_print_int

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
; ------------------------------------------------------------
; neko_print_str — write a NUL-terminated string to stdout
;   in:  rdi = pointer to the string
; ------------------------------------------------------------
neko_print_str:
    push rbp
    mov  rbp, rsp
    sub  rsp, 16

    mov  rsi, rdi              ; rsi = buffer
    mov  rdx, 0                ; rdx = length (computed below)

.len_loop:
    cmp  byte [rsi + rdx], 0   ; reached the NUL terminator?
    je   .len_done
    inc  rdx
    jmp  .len_loop

.len_done:
    mov  rax, 1                ; syscall: write
    mov  rdi, 1                ; fd = stdout
    syscall

    add  rsp, 16
    pop  rbp
    ret

; ------------------------------------------------------------
; neko_print_int — print a signed integer in decimal
;   in:  rdi = the value
; ------------------------------------------------------------
neko_print_int:
    push rbp
    mov  rbp, rsp
    sub  rsp, 16

    mov  rax, rdi              ; rax = the value
    mov  r8, 0                 ; r8 = negative flag

    test rax, rax
    jns  .nonneg
    mov  r8, 1
    neg  rax                   ; work with the absolute value

.nonneg:
    mov  rsi, numbuf_end       ; rsi = write pointer (moves backwards)

.digit_loop:
    mov  rcx, 10
    xor  rdx, rdx              ; rdx must be 0 before `div`
    div  rcx                   ; rax = quotient, rdx = remainder
    add  dl, '0'               ; remainder -> ASCII digit
    dec  rsi
    mov  [rsi], dl             ; store the digit
    test rax, rax
    jnz  .digit_loop

    test r8, r8
    jz   .write_it
    dec  rsi
    mov  byte [rsi], '-'        ; negative sign

.write_it:
    mov  rdx, numbuf_end
    sub  rdx, rsi              ; rdx = number of characters
    mov  rax, 1                ; syscall: write
    mov  rdi, 1                ; fd = stdout
    syscall

    add  rsp, 16
    pop  rbp
    ret

; ------------------------------------------------------------
; neko_print_bool — print "true" or "false"
;   in:  rdi = 0 for false, non-zero for true
; ------------------------------------------------------------
neko_print_bool:
    push rbp
    mov  rbp, rsp
    sub  rsp, 16

    test rdi, rdi
    jnz  .is_true

    lea  rsi, [rel str_false]
    mov  rdx, 5                ; length of "false"
    jmp  .write_it

.is_true:
    lea  rsi, [rel str_true]
    mov  rdx, 4                ; length of "true"

.write_it:
    mov  rax, 1                ; syscall: write
    mov  rdi, 1                ; fd = stdout
    syscall

    add  rsp, 16
    pop  rbp
    ret

; ------------------------------------------------------------
; neko_read_line — read a line from the keyboard into a buffer
;   in:  rdi = destination buffer
;         rsi = maximum number of bytes to read
;   out: rax = number of bytes read
; ------------------------------------------------------------
neko_read_line:
    push rbp
    mov  rbp, rsp
    sub  rsp, 16

    mov  rdx, rsi              ; rdx = count (max bytes)
    mov  rsi, rdi              ; rsi = buffer
    xor  rdi, rdi              ; rdi = fd 0 (stdin)
    mov  rax, 0                ; syscall: read
    syscall

    add  rsp, 16
    pop  rbp
    ret

