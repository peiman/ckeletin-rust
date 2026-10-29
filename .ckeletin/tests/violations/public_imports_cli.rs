// Violation test for CKSPEC-ARCH-007: a public crate must not import the CLI crate.
// This file MUST fail to compile. If it compiles, the boundary is broken.
//
// Copy it only when a declared cli package has a [lib] target or src/lib.rs.
// A bin-only cli has nothing to import: Cargo ignores that dependency, so
// `use cli::` fails whether or not the public crate lists it. Do not
// compile_fail this file in that project. The allowlist check is the rule.

use cli::root::Cli;

fn main() {
    let _ = std::any::type_name::<Cli>();
}
