// Violation test for CKSPEC-ARCH-007: a public crate must not import a domain crate.
// This file MUST fail to compile. If it compiles, the boundary is broken.

use domain::ping::PingResult;

fn main() {
    let _ = PingResult {
        message: "should not compile".to_string(),
    };
}
