use ckeletin_domain as domain;
use ckeletin_infrastructure::output::Output;
use std::io;

/// Execute the ping command through the output pipeline.
pub fn execute(output: &Output) -> io::Result<()> {
    let result = domain::ping::execute();
    output.success("ping", &result, &mut io::stdout())
}
