use serde::{Deserialize, Serialize};
use std::io::{self, Write};

/// Output status in the JSON envelope (CKSPEC-OUT-003).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Success,
    Error,
}

/// Standardized output envelope (CKSPEC-OUT-003).
///
/// Uses `serde_json::Value` for the data field so any Serialize type
/// can be wrapped without generics or lifetime complexity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Envelope {
    pub status: Status,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Envelope {
    /// Create a success envelope wrapping any serializable data.
    pub fn success<T: Serialize>(command: &str, data: &T) -> Result<Self, serde_json::Error> {
        Ok(Self {
            status: Status::Success,
            command: command.to_string(),
            data: Some(serde_json::to_value(data)?),
            error: None,
        })
    }

    /// Create an error envelope.
    pub fn error(command: &str, message: &str) -> Self {
        Self {
            status: Status::Error,
            command: command.to_string(),
            data: None,
            error: Some(message.to_string()),
        }
    }
}

/// Output mode selection (CKSPEC-OUT-002).
#[derive(Debug, Clone, PartialEq)]
pub enum OutputMode {
    Human,
    Json,
}

/// Renders command output to the appropriate streams.
///
/// Business logic MUST NOT write to stdout/stderr directly (CKSPEC-OUT-005).
/// All output goes through this struct. Writer injection for testability.
pub struct Output {
    mode: OutputMode,
}

impl Output {
    pub fn new(mode: OutputMode) -> Self {
        Self { mode }
    }

    /// Render success output to the provided writer.
    ///
    /// Human mode: Display format to stdout.
    /// JSON mode: envelope to stdout.
    /// Both modes: shadow log to audit stream (CKSPEC-OUT-004).
    pub fn success<T: Serialize + std::fmt::Display>(
        &self,
        command: &str,
        data: &T,
        out: &mut dyn Write,
    ) -> io::Result<()> {
        tracing::debug!(command = command, "output.success");
        match self.mode {
            OutputMode::Human => writeln!(out, "{data}"),
            OutputMode::Json => {
                let envelope = Envelope::success(command, data).map_err(io::Error::other)?;
                serde_json::to_writer_pretty(&mut *out, &envelope).map_err(io::Error::other)?;
                writeln!(out)
            }
        }
    }

    /// Render error output.
    ///
    /// Human mode: message to stderr writer.
    /// JSON mode: envelope to stdout writer.
    /// Both modes: shadow log to audit stream (CKSPEC-OUT-004).
    pub fn error(
        &self,
        command: &str,
        err_msg: &str,
        stdout: &mut dyn Write,
        stderr: &mut dyn Write,
    ) -> io::Result<()> {
        tracing::debug!(command = command, error = err_msg, "output.error");
        match self.mode {
            OutputMode::Human => writeln!(stderr, "Error: {err_msg}"),
            OutputMode::Json => {
                let envelope = Envelope::error(command, err_msg);
                serde_json::to_writer_pretty(&mut *stdout, &envelope).map_err(io::Error::other)?;
                writeln!(stdout)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    // --- Envelope tests ---

    #[test]
    fn envelope_success_serializes_correctly() {
        let data = serde_json::json!({"message": "pong"});
        let envelope = Envelope {
            status: Status::Success,
            command: "ping".to_string(),
            data: Some(data),
            error: None,
        };
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["status"], "success");
        assert_eq!(json["command"], "ping");
        assert_eq!(json["data"]["message"], "pong");
        // error field should be absent (skip_serializing_if)
        assert!(json.get("error").is_none());
    }

    #[test]
    fn envelope_error_serializes_correctly() {
        let envelope = Envelope::error("ping", "connection failed");
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["status"], "error");
        assert_eq!(json["command"], "ping");
        assert_eq!(json["error"], "connection failed");
        // data field should be absent
        assert!(json.get("data").is_none());
    }

    #[test]
    fn envelope_success_constructor_wraps_any_serializable() {
        #[derive(Serialize)]
        struct Custom {
            count: u32,
            name: String,
        }
        let data = Custom {
            count: 42,
            name: "test".to_string(),
        };
        let envelope = Envelope::success("cmd", &data).unwrap();
        assert_eq!(envelope.status, Status::Success);
        assert_eq!(envelope.data.as_ref().unwrap()["count"], 42);
        assert_eq!(envelope.data.as_ref().unwrap()["name"], "test");
        assert!(envelope.error.is_none());
    }

    #[test]
    fn envelope_error_constructor() {
        let envelope = Envelope::error("cmd", "timeout");
        assert_eq!(envelope.status, Status::Error);
        assert!(envelope.data.is_none());
        assert_eq!(envelope.error.as_ref().unwrap(), "timeout");
    }

    #[test]
    fn envelope_roundtrips_through_json() {
        let original = Envelope::success("ping", &"pong").unwrap();
        let json_str = serde_json::to_string(&original).unwrap();
        let parsed: Envelope = serde_json::from_str(&json_str).unwrap();
        assert_eq!(original, parsed);
    }

    // --- Renderer tests ---

    #[derive(Serialize)]
    struct TestData {
        value: String,
    }

    impl fmt::Display for TestData {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.value)
        }
    }

    fn test_data(s: &str) -> TestData {
        TestData {
            value: s.to_string(),
        }
    }

    #[test]
    fn human_success_writes_display_to_writer() {
        let output = Output::new(OutputMode::Human);
        let mut buf = Vec::new();
        output
            .success("test", &test_data("hello world"), &mut buf)
            .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "hello world\n");
    }

    #[test]
    fn json_success_writes_envelope_to_writer() {
        let output = Output::new(OutputMode::Json);
        let mut buf = Vec::new();
        output
            .success("ping", &test_data("hello"), &mut buf)
            .unwrap();
        let envelope: Envelope = serde_json::from_slice(&buf).unwrap();
        assert_eq!(envelope.status, Status::Success);
        assert_eq!(envelope.command, "ping");
        assert_eq!(envelope.data.unwrap()["value"], "hello");
    }

    #[test]
    fn human_error_writes_to_stderr_writer() {
        let output = Output::new(OutputMode::Human);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        output
            .error("test", "something broke", &mut stdout, &mut stderr)
            .unwrap();
        assert!(stdout.is_empty(), "human error should not write to stdout");
        assert_eq!(
            String::from_utf8(stderr).unwrap(),
            "Error: something broke\n"
        );
    }

    #[test]
    fn json_error_writes_envelope_to_stdout_writer() {
        let output = Output::new(OutputMode::Json);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        output
            .error("test", "something broke", &mut stdout, &mut stderr)
            .unwrap();
        assert!(stderr.is_empty(), "json error should not write to stderr");
        let envelope: Envelope = serde_json::from_slice(&stdout).unwrap();
        assert_eq!(envelope.status, Status::Error);
        assert_eq!(envelope.error.unwrap(), "something broke");
    }

    #[test]
    fn json_success_output_is_valid_parseable_json() {
        let output = Output::new(OutputMode::Json);
        let mut buf = Vec::new();
        output.success("cmd", &test_data("data"), &mut buf).unwrap();
        // Must parse as valid JSON
        let parsed: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn json_error_output_is_valid_parseable_json() {
        let output = Output::new(OutputMode::Json);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        output
            .error("cmd", "fail", &mut stdout, &mut stderr)
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
        assert!(parsed.is_object());
    }
}
