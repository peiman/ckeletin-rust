// Violation test for CKSPEC-ARCH-007: a public crate must not import infrastructure.
// This file MUST fail to compile. If it compiles, the boundary is broken.

use infrastructure::output::Output;

fn main() {
    let _ = Output::new(infrastructure::output::OutputMode::Human);
}
