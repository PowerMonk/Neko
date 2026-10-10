// ============================================================================
// CodeEmitter
//
// Centralized helper that owns:
//   - the growing list of quadruples
//   - the growing list of triples
//   - the counter for temporaries (`T1`, `T2`, ...)
//   - the counter for jump labels (`L1`, `L2`, ...)
//
// Why a separate struct?
// -----------------------
// The visitor (ICGenerator) is recursive. Each sub-expression needs to
// "ask for" a fresh temporary. Having a single emitter means the counter
// is global to the whole program, not per-expression — which is what we
// want so temporaries don't collide.
//
// The emitter does NOT know about the AST. It only exposes three
// operations:
//
//   1. `emit_quad(op, arg1, arg2, result)` — append a quadruple.
//   2. `emit_triple(op, arg1, arg2)`       — append a triple. Returns
//      the index of the new triple so the caller can reference it
//      from later triples (e.g. `(+ , a, (0))`).
//   3. `next_temp()` / `next_label()`      — return the next available
//      name (`T3`, `L4`, ...) WITHOUT consuming it; the caller can
//      decide what to do with it (often: pass it to emit_quad as the
//      `result` field, or remember it for a later GOTO).
//
// Token convention: temporaries start at 1 (so the first one is `T1`,
// not `T0`); labels start at 1 for the same reason.
// ============================================================================

use super::quadruple::Quadruple;
use super::triple::Triple;

pub struct CodeEmitter {
    quadruples: Vec<Quadruple>,
    triples: Vec<Triple>,
    temp_counter: usize,
    label_counter: usize,
}

impl CodeEmitter {
    pub fn new() -> Self {
        Self {
            quadruples: Vec::new(),
            triples: Vec::new(),
            temp_counter: 0,   // next_temp() returns T1 first
            label_counter: 0,  // next_label() returns L1 first
        }
    }

    // ------------------------------------------------------------------
    // Quadruples
    // ------------------------------------------------------------------

    pub fn emit_quad(
        &mut self,
        op: impl Into<String>,
        arg1: impl Into<String>,
        arg2: impl Into<String>,
        result: impl Into<String>,
    ) {
        self.quadruples.push(Quadruple::new(op, arg1, arg2, result));
    }

    pub fn quadruples(&self) -> &[Quadruple] {
        &self.quadruples
    }

    pub fn render_quadruples(&self) -> String {
        let mut out = String::new();
        for (i, q) in self.quadruples.iter().enumerate() {
            // Line number prefix to match the traditional 3-address-code layout.
            out.push_str(&format!("{:>3} {}\n", i, q.render()));
        }
        out
    }

    // ------------------------------------------------------------------
    // Triples
    // ------------------------------------------------------------------

    /// Append a triple and return its index (which the caller can pass
    /// back as a reference like `(0)`, `(1)`, etc.).
    pub fn emit_triple(
        &mut self,
        op: impl Into<String>,
        arg1: impl Into<String>,
        arg2: impl Into<String>,
    ) -> usize {
        let idx = self.triples.len();
        self.triples.push(Triple::new(op, arg1, arg2));
        idx
    }

    pub fn triples(&self) -> &[Triple] {
        &self.triples
    }

    pub fn render_triples(&self) -> String {
        let mut out = String::new();
        for (i, t) in self.triples.iter().enumerate() {
            out.push_str(&format!("{}\n", t.render(i)));
        }
        out
    }

    // ------------------------------------------------------------------
    // Fresh temporaries and labels
    // ------------------------------------------------------------------

    /// Return a fresh temporary name (`T1`, `T2`, ...) and bump the counter.
    pub fn next_temp(&mut self) -> String {
        self.temp_counter += 1;
        format!("T{}", self.temp_counter)
    }

    /// Return a fresh label name (`L1`, `L2`, ...) and bump the counter.
    pub fn next_label(&mut self) -> String {
        self.label_counter += 1;
        format!("L{}", self.label_counter)
    }

    /// Peek at the next temporary name WITHOUT consuming it.
    /// (Rarely needed; included for completeness / debugging.)
    #[allow(dead_code)]
    pub fn peek_next_temp(&self) -> String {
        format!("T{}", self.temp_counter + 1)
    }
}
