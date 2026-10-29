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
    str_0:         db "Perfect", 0
    str_1:         db "Excellent", 0
    str_2:         db "Fail", 0
    str_3:         db "Alto", 0
    str_4:         db "Bajo", 0

section .bss

    ; --- runtime: entrada del teclado y búferes ---
    stdin:         resb 256
    stdin_len:     resq 1
    stdout:        resq 1
    numbuf:        resb 256
    numbuf_end:    equ numbuf + 256

    ; --- variables del usuario ---
    var_a:         resq 1
    var_b:         resq 1
    var_c:         resq 1
    var_x:         resq 1
    var_x2:        resq 1
    var_total:     resq 1
    var_score:     resq 1
    var_result:    resq 1
    var_cond:      resq 1
    var_final:     resq 1

    ; --- temporales del compilador ---
    T1:            resq 1
    T2:            resq 1
    T3:            resq 1
    T4:            resq 1
    T5:            resq 1
    T6:            resq 1
    T7:            resq 1
    T8:            resq 1
    T9:            resq 1
    T10:           resq 1

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
    mov rax, 10
    mov [var_a], rax
    mov rax, 3
    mov [var_b], rax
    mov rax, 2
    mov [var_c], rax
    mov rax, [var_b]
    mov rcx, [var_c]
    imul rax, rcx
    mov [T1], rax
    mov rax, [var_a]
    mov rcx, [T1]
    add rax, rcx
    mov [T2], rax
    mov rax, [T2]
    mov [var_x], rax
    mov rax, [var_b]
    mov rcx, [var_c]
    imul rax, rcx
    mov [T3], rax
    mov rax, [T3]
    mov rcx, [var_a]
    add rax, rcx
    mov [T4], rax
    mov rax, [T4]
    mov [var_x2], rax
    mov rax, [var_x]
    mov rcx, [var_x2]
    add rax, rcx
    mov [T5], rax
    mov rax, [T5]
    mov [var_total], rax
    mov rax, 100
    mov [var_score], rax
    mov rax, [var_score]
    mov rcx, 100
    cmp rax, rcx
    sete al
    movzx rax, al
    mov [T7], rax
    mov rax, [T7]
    test rax, rax
    jnz L2
    mov rax, [var_score]
    mov rcx, 90
    cmp rax, rcx
    sete al
    movzx rax, al
    mov [T8], rax
    mov rax, [T8]
    test rax, rax
    jnz L3
    jmp L4
L2:
    lea rax, [rel str_0]
    mov [T6], rax
    jmp L1
L3:
    lea rax, [rel str_1]
    mov [T6], rax
    jmp L1
L4:
    lea rax, [rel str_2]
    mov [T6], rax
L1:
    mov rax, [T6]
    mov [var_result], rax
    mov rax, [var_total]
    mov rcx, 10
    cmp rax, rcx
    setg al
    movzx rax, al
    mov [T9], rax
    mov rax, [T9]
    mov [var_cond], rax
    mov rax, [var_cond]
    test rax, rax
    jz L5
    lea rax, [rel str_3]
    mov [T10], rax
    jmp L6
L5:
    lea rax, [rel str_4]
    mov [T10], rax
L6:
    mov rax, [T10]
    mov [var_final], rax
    mov rdi, [var_total]
    call neko_print_int
    mov rdi, [var_result]
    call neko_print_str
    mov rdi, [var_final]
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

