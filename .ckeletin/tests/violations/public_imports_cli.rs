// Violation test for CKSPEC-ARCH-007: a public crate must not import the CLI crate.
// This file MUST fail to compile. If it compiles, the boundary is broken.

use cli::root::Cli;

fn main() {
    let _ = std::any::type_name::<Cli>();
}
