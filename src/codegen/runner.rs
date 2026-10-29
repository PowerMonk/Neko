// ============================================================================
// Runner — assembles, links and runs the generated assembly program.
//
// The generated `.asm` targets Linux x86-64. On Windows we therefore
// need a Linux environment to execute it. This module drives WSL (Windows
// Subsystem for Linux) to do the three steps:
//
//     nasm -f elf64 program.asm -o program.o
//     ld program.o -o program
//     ./program
//
// If WSL is not available the compiler prints the manual commands
// instead, so the generated file is still useful on any Linux machine.
//
// Path translation
// ----------------
// WSL mounts the Windows drives under `/mnt/<drive>`, so
// `C:\Users\me\neko\output` becomes `/mnt/c/Users/me/neko/output`.
// `to_wsl_path` performs that translation.
// ============================================================================

use std::path::Path;
use std::process::Command;

/// The outcome of trying to build and run the generated program.
pub enum RunOutcome {
    /// The program ran; here is everything it wrote to stdout/stderr.
    Ran { output: String },
    /// The program could not be run; here is an explanation.
    Failed { reason: String },
}

/// Translate a Windows path into its WSL equivalent.
///
/// `C:\Users\me\out`  →  `/mnt/c/Users/me/out`
/// `\\wsl$\Ubuntu\home\me` is left alone (it is already a Linux path).
pub fn to_wsl_path(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");

    // Already a Linux-style path.
    if text.starts_with('/') {
        return text;
    }

    // Windows drive letter: `C:/...`
    let bytes: Vec<char> = text.chars().collect();
    if bytes.len() >= 3 && bytes[1] == ':' && bytes[2] == '/' {
        let drive = bytes[0].to_ascii_lowercase();
        let rest: String = bytes[3..].iter().collect();
        return format!("/mnt/{}/{}", drive, rest);
    }

    // Fallback: hand it to WSL unchanged and hope for the best.
    text
}

/// Assemble, link and run the program in `output_dir` via WSL.
///
/// `output_dir` must be the directory that holds `program.asm`.
pub fn build_and_run(output_dir: &Path) -> RunOutcome {
    // ---- Is WSL available? ----
    let probe = Command::new("wsl")
        .arg("--status")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match probe {
        Ok(status) if status.success() => {}
        Ok(_) => {
            return RunOutcome::Failed {
                reason: "WSL esta instalado pero no responde.".to_string(),
            }
        }
        Err(_) => {
            return RunOutcome::Failed {
                reason: "WSL no esta disponible en esta maquina.".to_string(),
            }
        }
    }

    // ---- Build the shell command that runs inside WSL ----
    let wsl_dir = to_wsl_path(output_dir);
    let script = format!(
        "set -e\n\
         cd {dir}\n\
         nasm -f elf64 program.asm -o program.o\n\
         ld program.o -o program\n\
         chmod +x program\n\
         ./program\n",
        dir = shell_quote(&wsl_dir)
    );

    // ---- Run it ----
    let result = Command::new("wsl")
        .arg("--")
        .arg("bash")
        .arg("-c")
        .arg(&script)
        .output();

    match result {
        Ok(output) => {
            let mut combined = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.trim().is_empty() {
                combined.push_str(&stderr);
            }
            if output.status.success() {
                RunOutcome::Ran { output: combined }
            } else {
                RunOutcome::Failed {
                    reason: format!(
                        "El programa ensamblado termino con error.\n{}",
                        combined.trim()
                    ),
                }
            }
        }
        Err(error) => RunOutcome::Failed {
            reason: format!("No se pudo invocar WSL: {}", error),
        },
    }
}

/// Quote a path for use inside a POSIX shell command.
fn shell_quote(path: &str) -> String {
    if path.contains('\'') {
        // Escape single quotes the POSIX way: close, add an escaped
        // quote, reopen.
        format!("'{}'", path.replace('\'', "'\\''"))
    } else {
        format!("'{}'", path)
    }
}

/// The manual instructions, printed when we cannot run the program.
pub fn manual_instructions(output_dir: &Path) -> String {
    let wsl_dir = to_wsl_path(output_dir);
    format!(
        "Para compilar y ejecutar el .asm manualmente (en Linux o WSL):\n\
         \n    cd {dir}\n    \
         nasm -f elf64 program.asm -o program.o\n    \
         ld program.o -o program\n    \
         ./program\n",
        dir = wsl_dir
    )
}
