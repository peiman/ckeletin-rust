use domain::ping;
use infrastructure::output::{ExitError, Output};

/// Execute the ping command through the output pipeline.
///
/// `Err(ExitError::new(message, code))` chooses the non-zero process exit.
/// This handler renders only success; `run` renders the one error envelope.
pub fn execute(output: &Output) -> Result<(), ExitError> {
    let result = ping::execute();
    let command = crate::command_id(&["ping"]);
    output.success(&command, &result, &mut std::io::stdout())?;
    Ok(())
}
