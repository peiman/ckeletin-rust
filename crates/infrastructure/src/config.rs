use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};

/// Application configuration.
///
/// Loaded with layered precedence: defaults < config file < env vars.
/// CLI flag overrides are applied by the cli crate after loading.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Console log level (trace, debug, info, warn, error).
    #[serde(default = "defaults::log_level")]
    pub log_level: String,

    /// Enable file logging (audit stream).
    #[serde(default)]
    pub log_file_enabled: bool,

    /// Path to the log file.
    #[serde(default = "defaults::log_file_path")]
    pub log_file_path: String,

    /// File log level.
    #[serde(default = "defaults::log_file_level")]
    pub log_file_level: String,

    /// Enable JSON output mode globally.
    #[serde(default)]
    pub json: bool,
}

mod defaults {
    pub fn log_level() -> String {
        "info".to_string()
    }
    pub fn log_file_path() -> String {
        "logs/ckeletin-rust.log".to_string()
    }
    pub fn log_file_level() -> String {
        "debug".to_string()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: defaults::log_level(),
            log_file_enabled: false,
            log_file_path: defaults::log_file_path(),
            log_file_level: defaults::log_file_level(),
            json: false,
        }
    }
}

impl Config {
    /// Load configuration with layered precedence:
    /// defaults → config file (if exists) → environment variables.
    ///
    /// Missing config file is not an error — defaults apply.
    /// figment's provenance tracking gives clear error messages
    /// on misconfiguration (stronger than Viper's interface{}).
    pub fn load(config_path: Option<&str>) -> Result<Self, Box<figment::Error>> {
        let mut figment = Figment::new().merge(Serialized::defaults(Config::default()));

        if let Some(path) = config_path {
            figment = figment.merge(Toml::file(path));
        } else {
            // Default location — missing file is silently ignored
            figment = figment.merge(Toml::file("config.toml"));
        }

        figment
            .merge(Env::prefixed("CKELETIN_").split("_"))
            .extract()
            .map_err(Box::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn default_config_values() {
        let config = Config::default();
        assert_eq!(config.log_level, "info");
        assert!(!config.log_file_enabled);
        assert_eq!(config.log_file_path, "logs/ckeletin-rust.log");
        assert_eq!(config.log_file_level, "debug");
        assert!(!config.json);
    }

    #[test]
    fn load_returns_defaults_when_no_file() {
        let config = Config::load(None).unwrap();
        assert_eq!(config.log_level, "info");
        assert!(!config.json);
    }

    #[test]
    fn load_reads_toml_file() {
        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        writeln!(file, "log_level = \"debug\"\njson = true").unwrap();
        let config = Config::load(Some(file.path().to_str().unwrap())).unwrap();
        assert_eq!(config.log_level, "debug");
        assert!(config.json);
    }

    #[test]
    fn toml_overrides_only_specified_values() {
        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        writeln!(file, "log_file_enabled = true").unwrap();
        let config = Config::load(Some(file.path().to_str().unwrap())).unwrap();
        assert!(config.log_file_enabled);
        // Unspecified values remain default
        assert_eq!(config.log_level, "info");
        assert!(!config.json);
    }

    #[test]
    fn invalid_toml_returns_error() {
        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        writeln!(file, "not valid toml [[[").unwrap();
        let result = Config::load(Some(file.path().to_str().unwrap()));
        assert!(result.is_err());
    }
}
