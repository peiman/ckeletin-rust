use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

/// Application configuration.
///
/// Loaded with layered precedence: framework defaults < `[defaults]` in the
/// repo-root `ckeletin-project.toml` < config file < env vars.
/// CLI flag overrides are applied by the cli crate after loading.
///
/// `[defaults]` is embedded from that file at compile time, so an installed
/// binary keeps the project default. The file lives outside `.ckeletin/`, so a
/// framework update does not delete it. Absent `[defaults]` changes nothing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Console log level (trace, debug, info, warn, error).
    #[serde(default = "defaults::log_level")]
    pub log_level: String,

    /// Enable file logging (audit stream). On by default (CKSPEC-OUT-004:
    /// the audit stream is always active unless explicitly disabled).
    #[serde(default = "defaults::log_file_enabled")]
    pub log_file_enabled: bool,

    /// Path to the log file.
    #[serde(default = "defaults::log_file_path")]
    pub log_file_path: String,

    /// File log level.
    #[serde(default = "defaults::log_file_level")]
    pub log_file_level: String,

    /// Where the audit log lives when `log_file_path` is relative:
    /// "config" → ~/.config/<app> (default, XDG-style on every platform),
    /// "platform" → the OS-native app-data dir (e.g. ~/Library/Application
    /// Support/<app> on macOS). An absolute `log_file_path` overrides this.
    #[serde(default = "defaults::log_location")]
    pub log_location: String,

    /// When true, a failure to open or write the audit file warns once and
    /// the command continues. Default false: logging init fails closed.
    #[serde(default = "defaults::log_file_best_effort")]
    pub log_file_best_effort: bool,

    /// Enable JSON output mode globally.
    #[serde(default)]
    pub json: bool,
}

mod defaults {
    pub fn log_level() -> String {
        "info".to_string()
    }
    pub fn log_file_enabled() -> bool {
        true
    }
    pub fn log_file_path() -> String {
        "logs/app.log".to_string()
    }
    pub fn log_file_level() -> String {
        "debug".to_string()
    }
    pub fn log_location() -> String {
        "config".to_string()
    }
    pub fn log_file_best_effort() -> bool {
        false
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: defaults::log_level(),
            log_file_enabled: defaults::log_file_enabled(),
            log_file_path: defaults::log_file_path(),
            log_file_level: defaults::log_file_level(),
            log_location: defaults::log_location(),
            log_file_best_effort: defaults::log_file_best_effort(),
            json: false,
        }
    }
}

/// Repo-root project file, relative to the framework crate directory.
/// `include_str!` below is relative to this source file (`src/`), one level deeper.
#[cfg(test)]
const PROJECT_DEFAULTS_FROM_CRATE: &str = "../../ckeletin-project.toml";

/// Compile-time copy of the repo-root `ckeletin-project.toml`.
const EMBEDDED_PROJECT_TOML: &str = include_str!("../../../ckeletin-project.toml");

/// Keys a project may set under `[defaults]`. They are `Config` field names.
const PROJECT_DEFAULT_KEYS: &[&str] = &[
    "log_level",
    "log_file_enabled",
    "log_file_path",
    "log_file_level",
    "log_location",
    "log_file_best_effort",
    "json",
];

/// The project file has other tables (`layers`, `allowlists`, `violation_tests`).
/// This loader reads only `[defaults]` and ignores the rest.
#[derive(Debug, Default, Deserialize)]
struct ProjectFile {
    #[serde(default)]
    defaults: Option<figment::value::Dict>,
}

#[derive(Debug, Default, Serialize)]
struct ProjectDefaults {
    #[serde(skip_serializing_if = "Option::is_none")]
    log_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_file_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_file_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_file_best_effort: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    json: Option<bool>,
}

impl Config {
    /// Load configuration with layered precedence:
    /// framework defaults → `[defaults]` → config file (if exists) → environment variables.
    ///
    /// `env_prefix` controls which environment variables are read.
    /// Projects pass their own name: `"WORKHORSE_"`, `"MYAPP_"`.
    /// The scaffold default is `"CKELETIN_"`.
    ///
    /// Missing config file is not an error — defaults apply.
    /// An unknown key in `[defaults]` is an error.
    pub fn load(
        config_path: Option<&str>,
        env_prefix: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::load_with_project_toml(config_path, env_prefix, EMBEDDED_PROJECT_TOML)
    }

    fn load_with_project_toml(
        config_path: Option<&str>,
        env_prefix: &str,
        project_toml: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut figment = Figment::new().merge(Serialized::defaults(Config::default()));
        if let Some(defaults) = project_defaults(project_toml)? {
            figment = figment.merge(Serialized::defaults(defaults));
        }

        if let Some(path) = config_path {
            let p = std::path::Path::new(path);
            // Explicit --config: file MUST exist and MUST be a regular file.
            // Silent fallback on not-found or is-a-directory is misleading.
            if !p.exists() {
                return Err(format!("config file not found: {path}").into());
            }
            if p.is_dir() {
                return Err(format!("config path is a directory, not a file: {path}").into());
            }
            figment = figment.merge(Toml::file(path));
        } else {
            // Default location — missing file is silently ignored
            figment = figment.merge(Toml::file("config.toml"));
        }

        figment
            .merge(Env::prefixed(env_prefix))
            .extract()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

fn project_defaults(
    project_toml: &str,
) -> Result<Option<ProjectDefaults>, Box<dyn std::error::Error>> {
    let file: ProjectFile = Figment::new()
        .merge(Toml::string(project_toml))
        .extract()
        .map_err(|e| format!("ckeletin-project.toml: {e}"))?;
    let Some(dict) = file.defaults else {
        return Ok(None);
    };
    for key in dict.keys() {
        if !PROJECT_DEFAULT_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "unknown project default `{key}` in ckeletin-project.toml [defaults]"
            )
            .into());
        }
    }
    let mut defaults = ProjectDefaults::default();
    for (key, value) in dict {
        match key.as_str() {
            "log_level" => defaults.log_level = Some(value_as_string(&value, &key)?),
            "log_file_enabled" => {
                defaults.log_file_enabled = Some(value_as_bool(&value, &key)?);
            }
            "log_file_path" => defaults.log_file_path = Some(value_as_string(&value, &key)?),
            "log_file_level" => defaults.log_file_level = Some(value_as_string(&value, &key)?),
            "log_location" => defaults.log_location = Some(value_as_string(&value, &key)?),
            "log_file_best_effort" => {
                defaults.log_file_best_effort = Some(value_as_bool(&value, &key)?);
            }
            "json" => defaults.json = Some(value_as_bool(&value, &key)?),
            _ => {
                return Err(format!(
                    "unknown project default `{key}` in ckeletin-project.toml [defaults]"
                )
                .into());
            }
        }
    }
    Ok(Some(defaults))
}

fn value_as_string(
    value: &figment::value::Value,
    key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    value.clone().into_string().ok_or_else(|| {
        format!("project default `{key}` in ckeletin-project.toml [defaults] must be a string")
            .into()
    })
}

fn value_as_bool(
    value: &figment::value::Value,
    key: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    value.to_bool().ok_or_else(|| {
        format!("project default `{key}` in ckeletin-project.toml [defaults] must be a boolean")
            .into()
    })
}

#[cfg(test)]
#[allow(clippy::result_large_err)] // figment::Error is large; acceptable in test code
mod tests {
    use super::*;
    use figment::Jail;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const TEST_PREFIX: &str = "CKTEST_";

    #[test]
    fn default_config_values() {
        let config = Config::default();
        assert_eq!(config.log_level, "info");
        assert!(config.log_file_enabled, "audit log is on by default");
        assert_eq!(config.log_file_path, "logs/app.log");
        assert_eq!(config.log_file_level, "debug");
        assert_eq!(config.log_location, "config");
        assert!(!config.log_file_best_effort, "audit log stays fail-closed");
        assert!(!config.json);
    }

    #[test]
    fn load_returns_defaults_when_no_file() {
        // Use Jail to ensure there is no config.toml in cwd, even when other
        // Jail tests run in parallel and temporarily change the working directory.
        Jail::expect_with(|_jail| {
            let config = load_in_jail(None, TEST_PREFIX)?;
            assert_eq!(config.log_level, "info");
            assert!(!config.json);
            Ok(())
        });
    }

    #[test]
    fn load_reads_toml_file() {
        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        writeln!(file, "log_level = \"debug\"\njson = true").unwrap();
        let config = Config::load(Some(file.path().to_str().unwrap()), TEST_PREFIX).unwrap();
        assert_eq!(config.log_level, "debug");
        assert!(config.json);
    }

    #[test]
    fn toml_overrides_only_specified_values() {
        // Audit logging defaults to on; a TOML value can turn it off without
        // disturbing the other defaults.
        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        writeln!(file, "log_file_enabled = false").unwrap();
        let config = Config::load(Some(file.path().to_str().unwrap()), TEST_PREFIX).unwrap();
        assert!(!config.log_file_enabled);
        assert_eq!(config.log_level, "info");
        assert!(!config.json);
    }

    #[test]
    fn invalid_toml_returns_error() {
        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        writeln!(file, "not valid toml [[[").unwrap();
        let result = Config::load(Some(file.path().to_str().unwrap()), TEST_PREFIX);
        assert!(result.is_err());
    }

    #[test]
    fn explicit_missing_file_returns_error() {
        let result = Config::load(Some("/nonexistent/config.toml"), TEST_PREFIX);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found"),
            "Error should mention file not found: {err}"
        );
    }

    #[test]
    fn explicit_config_pointing_at_directory_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let result = Config::load(Some(dir.path().to_str().unwrap()), TEST_PREFIX);
        assert!(
            result.is_err(),
            "Expected error when --config points at a directory, got Ok"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("directory") || err.contains("not a file"),
            "Error message should mention directory or 'not a file': {err}"
        );
    }

    /// Helper: map Config::load's Box<dyn Error> into figment::Error for Jail closures.
    fn load_in_jail(config_path: Option<&str>, prefix: &str) -> figment::Result<Config> {
        Config::load(config_path, prefix).map_err(|e| e.to_string().into())
    }

    /// Env prefix is respected — uses figment::Jail for race-free env isolation.
    #[test]
    fn env_prefix_is_respected() {
        Jail::expect_with(|jail| {
            jail.set_env("CKROBUST_LOG_LEVEL", "trace");
            let config = load_in_jail(None, "CKROBUST_")?;
            assert_eq!(config.log_level, "trace");
            Ok(())
        });
    }

    /// Wrong prefix env vars are ignored — uses figment::Jail for isolation.
    #[test]
    fn different_prefix_ignores_other_env_vars() {
        Jail::expect_with(|jail| {
            jail.set_env("WRONGPREFIX_LOG_LEVEL", "error");
            let config = load_in_jail(None, "RIGHTPREFIX_")?;
            assert_eq!(config.log_level, "info");
            Ok(())
        });
    }

    /// Every config field must be settable via env var.
    /// This catches the .split("_") bug that silently broke env overrides.
    /// Uses figment::Jail for serialised, race-free env+fs isolation.
    #[test]
    fn every_config_field_settable_via_env() {
        Jail::expect_with(|jail| {
            jail.set_env("CKEVERY_LOG_LEVEL", "error");
            jail.set_env("CKEVERY_LOG_FILE_ENABLED", "true");
            jail.set_env("CKEVERY_LOG_FILE_PATH", "/tmp/test.log");
            jail.set_env("CKEVERY_LOG_FILE_LEVEL", "trace");
            jail.set_env("CKEVERY_LOG_LOCATION", "platform");
            jail.set_env("CKEVERY_LOG_FILE_BEST_EFFORT", "true");
            jail.set_env("CKEVERY_JSON", "true");

            let config = load_in_jail(None, "CKEVERY_")?;
            assert_eq!(config.log_level, "error", "LOG_LEVEL env not applied");
            assert!(config.log_file_enabled, "LOG_FILE_ENABLED env not applied");
            assert_eq!(
                config.log_file_path, "/tmp/test.log",
                "LOG_FILE_PATH env not applied"
            );
            assert_eq!(
                config.log_file_level, "trace",
                "LOG_FILE_LEVEL env not applied"
            );
            assert_eq!(
                config.log_location, "platform",
                "LOG_LOCATION env not applied"
            );
            assert!(
                config.log_file_best_effort,
                "LOG_FILE_BEST_EFFORT env not applied"
            );
            assert!(config.json, "JSON env not applied");
            Ok(())
        });
    }

    /// Env vars override TOML file values (precedence: default < file < env).
    /// Uses figment::Jail for serialised, race-free env+fs isolation.
    #[test]
    fn env_overrides_toml_file() {
        Jail::expect_with(|jail| {
            // Create a TOML file in the jail's temp directory
            jail.create_file("config.toml", "log_level = \"debug\"\n")?;
            jail.set_env("CKPREC_LOG_LEVEL", "error");

            // Pass the absolute path to the file created in the jail
            let config_path = jail.directory().join("config.toml");
            let config = load_in_jail(Some(config_path.to_str().unwrap()), "CKPREC_")?;
            assert_eq!(config.log_level, "error", "env should override TOML");
            Ok(())
        });
    }

    fn load_with_defaults(
        project_toml: &str,
        config_path: Option<&str>,
        prefix: &str,
    ) -> figment::Result<Config> {
        Config::load_with_project_toml(config_path, prefix, project_toml)
            .map_err(|e| e.to_string().into())
    }

    #[test]
    fn project_defaults_file_is_outside_framework_tree() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(super::PROJECT_DEFAULTS_FROM_CRATE);
        let canon = path
            .canonicalize()
            .unwrap_or_else(|e| panic!("repo-root ckeletin-project.toml ({path:?}): {e}"));
        assert!(
            !canon.components().any(|c| c.as_os_str() == ".ckeletin"),
            "project defaults file must not live under .ckeletin: {}",
            canon.display()
        );
        let disk = std::fs::read_to_string(&canon).unwrap();
        assert_eq!(
            disk,
            super::EMBEDDED_PROJECT_TOML,
            "Config embeds the repo-root ckeletin-project.toml"
        );
    }

    #[test]
    fn project_default_log_level_applies_when_nothing_else_sets_it() {
        Jail::expect_with(|_jail| {
            let config =
                load_with_defaults("[defaults]\nlog_level = \"warn\"\n", None, TEST_PREFIX)?;
            assert_eq!(config.log_level, "warn");
            assert!(config.log_file_enabled);
            assert!(!config.log_file_best_effort);
            assert!(!config.json);
            Ok(())
        });
    }

    #[test]
    fn project_default_can_opt_into_best_effort_audit_log() {
        Jail::expect_with(|_jail| {
            let config = load_with_defaults(
                "[defaults]\nlog_file_best_effort = true\n",
                None,
                TEST_PREFIX,
            )?;
            assert!(config.log_file_best_effort);
            assert!(config.log_file_enabled);
            Ok(())
        });
    }

    #[test]
    fn absent_defaults_table_keeps_framework_default() {
        Jail::expect_with(|_jail| {
            let config = load_with_defaults(
                "[layers]\ndomain = [\"crates/domain\"]\n",
                None,
                TEST_PREFIX,
            )?;
            assert_eq!(config.log_level, "info");
            Ok(())
        });
    }

    #[test]
    fn config_file_overrides_project_default() {
        Jail::expect_with(|jail| {
            jail.create_file("app.toml", "log_level = \"info\"\n")?;
            let path = jail.directory().join("app.toml");
            let config = load_with_defaults(
                "[defaults]\nlog_level = \"warn\"\n",
                Some(path.to_str().unwrap()),
                TEST_PREFIX,
            )?;
            assert_eq!(config.log_level, "info");
            Ok(())
        });
    }

    #[test]
    fn env_overrides_project_default() {
        Jail::expect_with(|jail| {
            jail.set_env("CKLAYER_LOG_LEVEL", "error");
            let config =
                load_with_defaults("[defaults]\nlog_level = \"warn\"\n", None, "CKLAYER_")?;
            assert_eq!(config.log_level, "error");
            Ok(())
        });
    }

    #[test]
    fn unknown_project_default_key_fails_load() {
        Jail::expect_with(|_jail| {
            let err = Config::load_with_project_toml(
                None,
                TEST_PREFIX,
                "[defaults]\nnot_a_real_option = \"warn\"\n",
            )
            .unwrap_err()
            .to_string();
            assert!(
                err.contains("not_a_real_option"),
                "error should name the unknown key: {err}"
            );
            Ok(())
        });
    }
}
