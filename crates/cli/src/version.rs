//! `version` command — the worked example of consuming ckeletin's build-identity
//! primitive. It mirrors the `ping` idiom (a thin CLI handler rendering a
//! `Serialize + Display` type through `Output` in both human and JSON modes); the
//! difference is that `ping` teaches "your own domain type" while this teaches
//! "a framework primitive". The env reads in `current()` are deliberately
//! explicit — not hidden behind a macro — so an adopter can see exactly how the
//! values baked by `build.rs` are wired into [`BuildInfo`].

use infrastructure::build_info::{BuildInfo, UNKNOWN};
use infrastructure::output::Output;
use std::io;

/// The running binary's build identity, from the values `build.rs` baked.
///
/// `option_env!` (not `env!`) so a binary built WITHOUT the build script still
/// compiles and renders an honest "unknown" — never a fabricated commit.
pub fn current() -> BuildInfo {
    BuildInfo::new(
        env!("CARGO_PKG_VERSION"),
        option_env!("CKELETIN_BUILD_COMMIT").unwrap_or(UNKNOWN),
        option_env!("CKELETIN_BUILD_DATE").unwrap_or(UNKNOWN),
        matches!(option_env!("CKELETIN_BUILD_DIRTY"), Some("true")),
    )
}

/// Execute the `version` command through the output pipeline.
pub fn execute(output: &Output) -> io::Result<()> {
    output.success("version", &current(), &mut io::stdout())
}
