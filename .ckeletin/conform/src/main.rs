use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::Command;

// ── Spec requirements source ──────────────────────────────────

const REQUIREMENTS_JSON_URL: &str =
    "https://raw.githubusercontent.com/peiman/ckeletin/main/spec/requirements.json";
const VENDORED_REQUIREMENTS: &str = "conformance/requirements.json";

#[derive(Deserialize)]
struct SpecManifest {
    spec_version: String,
    requirements: Vec<SpecRequirement>,
}

#[derive(Deserialize)]
struct SpecRequirement {
    id: String,
}

// ── Mapping file types (read from TOML) ─────────────────────────

#[derive(Deserialize)]
struct Mapping {
    spec_version: String,
    requirements: BTreeMap<String, RequirementMapping>,
}

#[derive(Deserialize)]
struct RequirementMapping {
    title: String,
    status: String,
    enforcement_level: String,
    evidence: String,
    #[serde(default)]
    checks: Vec<String>,
    #[serde(default)]
    violation_tests: Vec<String>,
    #[serde(default)]
    violation_evidence: Option<String>,
}

// ── Report types (output as JSON) ───────────────────────────────

#[derive(Serialize)]
struct Report {
    implementation: String,
    spec_version: String,
    report_date: String,
    summary: Summary,
    requirements: BTreeMap<String, RequirementResult>,
    feedback: Vec<String>,
}

#[derive(Serialize)]
struct Summary {
    total: usize,
    met: usize,
    partial: usize,
    deferred: usize,
    failed_checks: usize,
    feedback_signals: usize,
}

#[derive(Serialize)]
struct RequirementResult {
    title: String,
    status: String,
    enforcement_level: String,
    evidence: String,
    checks: Vec<CheckResult>,
    violation_tests: Vec<ViolationTestResult>,
}

#[derive(Serialize)]
struct CheckResult {
    command: String,
    passed: bool,
}

#[derive(Serialize)]
struct ViolationTestResult {
    path: String,
    exists: bool,
}

// ── Requirement ID loading (replaces hardcoded list) ────────────

/// Load the spec requirement IDs. Strategy:
/// 1. Fetch latest requirements.json from the spec repo
/// 2. If fetch succeeds, cache it to the vendored path and use it
/// 3. If fetch fails, fall back to the vendored copy with a warning
/// 4. If vendored copy also missing, abort
fn load_spec_requirements(json_mode: bool) -> (Vec<String>, String) {
    // Try fetching from upstream
    match fetch_upstream() {
        Ok(manifest) => {
            // Cache the fetched copy for offline use
            if let Ok(json) = serde_json::to_string_pretty(&serde_json::json!({
                "spec_version": manifest.spec_version,
                "requirements": manifest.requirements.iter().map(|r| {
                    serde_json::json!({"id": r.id})
                }).collect::<Vec<_>>()
            })) {
                let _ = std::fs::write(VENDORED_REQUIREMENTS, format!("{json}\n"));
            }
            let ids = manifest.requirements.iter().map(|r| r.id.clone()).collect();
            (ids, manifest.spec_version)
        }
        Err(fetch_err) => {
            // Fall back to vendored copy
            match load_vendored() {
                Ok(manifest) => {
                    if !json_mode {
                        eprintln!(
                            "Warning: could not fetch latest requirements ({fetch_err}). Using vendored copy (spec {}).",
                            manifest.spec_version
                        );
                    }
                    let ids = manifest.requirements.iter().map(|r| r.id.clone()).collect();
                    (ids, manifest.spec_version)
                }
                Err(vendor_err) => {
                    eprintln!("Error: cannot load spec requirements.");
                    eprintln!("  Fetch failed: {fetch_err}");
                    eprintln!("  Vendored copy: {vendor_err}");
                    eprintln!("  Run: curl -sL {REQUIREMENTS_JSON_URL} > {VENDORED_REQUIREMENTS}");
                    std::process::exit(1);
                }
            }
        }
    }
}

fn fetch_upstream() -> Result<SpecManifest, String> {
    let body: Vec<u8> = ureq::get(REQUIREMENTS_JSON_URL)
        .call()
        .map_err(|e| format!("{e}"))?
        .body_mut()
        .read_to_vec()
        .map_err(|e| format!("{e}"))?;
    serde_json::from_slice(&body).map_err(|e| format!("parse error: {e}"))
}

fn load_vendored() -> Result<SpecManifest, String> {
    let content = std::fs::read_to_string(VENDORED_REQUIREMENTS).map_err(|e| format!("{e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("parse error: {e}"))
}

fn main() {
    let json_mode = std::env::args().any(|a| a == "--json");

    let mapping_content = match std::fs::read_to_string("conformance-mapping.toml") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: cannot read conformance-mapping.toml: {e}");
            std::process::exit(1);
        }
    };

    let mapping: Mapping = match toml::from_str(&mapping_content) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: invalid mapping file: {e}");
            std::process::exit(1);
        }
    };

    // ── Load requirement IDs from spec (replaces hardcoded list) ──
    let (expected_ids, spec_version) = load_spec_requirements(json_mode);

    // ── Spec version comparison ────────────────────────────────
    if mapping.spec_version != spec_version && !json_mode {
        eprintln!(
            "Warning: mapping targets spec {} but requirements.json is spec {}",
            mapping.spec_version, spec_version
        );
    }

    // ── ENF-005: Completeness check ─────────────────────────────
    let missing: Vec<&str> = expected_ids
        .iter()
        .filter(|id| !mapping.requirements.contains_key(id.as_str()))
        .map(|s| s.as_str())
        .collect();

    if !missing.is_empty() {
        if json_mode {
            let err = serde_json::json!({
                "status": "error",
                "error": format!("unmapped requirements: {}", missing.join(", ")),
            });
            println!("{}", serde_json::to_string_pretty(&err).unwrap());
        } else {
            eprintln!("FAILED — unmapped requirements (CKSPEC-ENF-005 violation):");
            for m in &missing {
                eprintln!("  - {m}");
            }
        }
        std::process::exit(1);
    }

    // ── Run checks and collect results ──────────────────────────
    let mut results = BTreeMap::new();
    let mut feedback = Vec::new();
    let mut met = 0usize;
    let mut partial = 0usize;
    let mut deferred = 0usize;
    let mut failed_checks = 0usize;

    for (req_id, req) in &mapping.requirements {
        let mut check_results = Vec::new();
        let mut vtest_results = Vec::new();

        // Run checks
        for check_cmd in &req.checks {
            let passed = run_check(check_cmd);
            if !passed {
                failed_checks += 1;
            }
            if !json_mode {
                let icon = if passed { "ok" } else { "FAIL" };
                println!("  {req_id:<20} {check_cmd} ... {icon}");
            }
            check_results.push(CheckResult {
                command: check_cmd.clone(),
                passed,
            });
        }

        // Verify violation tests exist (ENF-006)
        for vt in &req.violation_tests {
            let exists = std::path::Path::new(vt).exists();
            if !exists {
                feedback.push(format!("{req_id}: violation test not found: {vt}"));
            }
            vtest_results.push(ViolationTestResult {
                path: vt.clone(),
                exists,
            });
        }

        // ENF-006: claims above honor-system need proof (violation_tests or violation_evidence)
        let above_honor = !matches!(req.enforcement_level.as_str(), "honor-system" | "design");
        let has_violation_test = !req.violation_tests.is_empty();
        let has_violation_evidence = req
            .violation_evidence
            .as_ref()
            .is_some_and(|e| !e.is_empty());
        if above_honor && !has_violation_test && !has_violation_evidence {
            feedback.push(format!(
                "{req_id}: claims {} but has no violation test or evidence",
                req.enforcement_level
            ));
        }

        match req.status.as_str() {
            "met" => met += 1,
            "partial" => partial += 1,
            "deferred" => deferred += 1,
            _ => {}
        }

        results.insert(
            req_id.clone(),
            RequirementResult {
                title: req.title.clone(),
                status: req.status.clone(),
                enforcement_level: req.enforcement_level.clone(),
                evidence: req.evidence.clone(),
                checks: check_results,
                violation_tests: vtest_results,
            },
        );
    }

    let total = mapping.requirements.len();
    let today = chrono_free_date();

    let report = Report {
        implementation: detect_implementation_name(),
        spec_version: mapping.spec_version.clone(),
        report_date: today,
        summary: Summary {
            total,
            met,
            partial,
            deferred,
            failed_checks,
            feedback_signals: feedback.len(),
        },
        requirements: results,
        feedback,
    };

    // ── Output ──────────────────────────────────────────────────

    if json_mode {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!();
        println!("── Results ──────────────────────────────────────────");
        println!();
        println!("  Requirements:  {} total", report.summary.total);
        println!("  Met:           {}", report.summary.met);
        if report.summary.partial > 0 {
            println!("  Partial:       {}", report.summary.partial);
        }
        if report.summary.deferred > 0 {
            println!("  Deferred:      {}", report.summary.deferred);
        }
        println!("  Failed checks: {}", report.summary.failed_checks);
        println!();

        if !report.feedback.is_empty() {
            println!("Feedback signals (ENF-007):");
            for f in &report.feedback {
                println!("  - {f}");
            }
            println!();
        }

        if report.summary.failed_checks > 0 {
            println!(
                "FAILED — {} check(s) did not pass.",
                report.summary.failed_checks
            );
            std::process::exit(1);
        }

        println!(
            "PASSED — {}/{} requirements met, {} deferred.",
            report.summary.met, report.summary.total, report.summary.deferred
        );
        if !report.feedback.is_empty() {
            println!(
                "         {} feedback signal(s) for spec review.",
                report.feedback.len()
            );
        }
    }

    if report.summary.failed_checks > 0 {
        std::process::exit(1);
    }
}

fn run_check(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Detect project name from the [[bin]] name in crates/cli/Cargo.toml.
fn detect_implementation_name() -> String {
    let content = match std::fs::read_to_string("crates/cli/Cargo.toml") {
        Ok(c) => c,
        Err(_) => return "unknown".to_string(),
    };
    let parsed: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return "unknown".to_string(),
    };
    // Read from [[bin]] array, first entry's name
    parsed
        .get("bin")
        .and_then(|b| b.as_array())
        .and_then(|arr| arr.first())
        .and_then(|entry| entry.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Simple date without chrono dependency.
fn chrono_free_date() -> String {
    let output = Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .expect("date command failed");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
