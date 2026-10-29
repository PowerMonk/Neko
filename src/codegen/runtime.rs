// ============================================================================
// Neko Runtime Routines
//
// The runtime support that every generated Neko program needs. These
// routines are emitted INTO the generated `.asm` file, so the result is a
// single self-contained file that can be assembled and run on its own.
//
// The teacher's notes mention "the in and out interruptions" — these are
// the Linux x86-64 system calls used here:
//
//   ┌──────────┬─────┬──────────────────────────────────────────┐
//   │ syscall  │ rax │ arguments                                │
//   ├──────────┼─────┼──────────────────────────────────────────┤
//   │ read     │  0  │ rdi=fd, rsi=buffer, rdx=count            │
//   │ write    │  1  │ rdi=fd, rsi=buffer, rdx=count            │
//   │ exit     │ 60  │ rdi=exit code                            │
//   └──────────┴─────┴──────────────────────────────────────────┘
//
// On Linux, `read` with fd 0 reads from the keyboard and `write` with
// fd 1 writes to the screen. Those are the "input and output
// interruptions" of the phase.
//
// Calling convention
// ------------------
// These routines are called with `call`, so they must respect the
// System V AMD64 ABI:
//   - `rsp` must be 16-byte aligned at the call site.
//   - `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9` carry the arguments.
//   - `rax` carries the return value.
//   - `rbx`, `rbp`, `r12`–`r15` are callee-saved.
//
// Stack alignment inside a routine:
//   At entry `rsp ≡ 8 (mod 16)` (the return address was pushed).
//   `push rbp` → `rsp ≡ 0 (mod 16)`.
//   `sub rsp, 16` → `rsp ≡ 0 (mod 16)`  ✓ aligned for any nested call.
//
// Note: the `syscall` instruction itself does NOT require 16-byte
// alignment (that rule only applies to `call`), but we keep the frame
// aligned anyway so the routines are safe to call from anywhere.
// ============================================================================

/// The `neko_print_str` routine: write a NUL-terminated string to stdout.
///
/// Input:  rdi = pointer to a NUL-terminated string
/// Output: nothing on screen beyond the string itself
///
/// The length is computed by scanning for the NUL terminator, which is
/// why every string in `.data` is stored with a trailing zero.
pub const PRINT_STR: &str = r#"; ------------------------------------------------------------
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
"#;

/// The `neko_print_int` routine: print a signed integer in decimal.
///
/// Input:  rdi = the value (signed 64-bit)
///
/// The decimal digits are built backwards into `numbuf` (starting at its
/// end and moving towards its start), then written out in one `write`.
///
/// `r8` holds the "was negative" flag because `div` clobbers `rdx`.
pub const PRINT_INT: &str = r#"; ------------------------------------------------------------
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
"#;

/// The `neko_print_bool` routine: print `true` or `false`.
///
/// Input:  rdi = 0 (false) or non-zero (true)
pub const PRINT_BOOL: &str = r#"; ------------------------------------------------------------
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
"#;

/// The `neko_read_line` routine: read a line from the keyboard.
///
/// Input:  rdi = destination buffer, rsi = maximum bytes to read
/// Output: rax = number of bytes actually read
///
/// This is the "input interruption": `read` with fd 0 reads from the
/// keyboard. The bytes land in the buffer that the symbol table
/// registers as `stdin`.
pub const READ_LINE: &str = r#"; ------------------------------------------------------------
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
"#;

/// All the runtime routines, in the order they are emitted.
pub const ALL_ROUTINES: &[&str] = &[PRINT_STR, PRINT_INT, PRINT_BOOL, READ_LINE];
