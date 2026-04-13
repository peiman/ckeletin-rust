use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::Command;

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

// ── Known spec requirement IDs (CKSPEC-ENF-005 completeness anchor) ──

const EXPECTED_IDS: &[&str] = &[
    "CKSPEC-ARCH-001",
    "CKSPEC-ARCH-002",
    "CKSPEC-ARCH-003",
    "CKSPEC-ARCH-004",
    "CKSPEC-ARCH-005",
    "CKSPEC-ARCH-006",
    "CKSPEC-ARCH-007",
    "CKSPEC-ENF-001",
    "CKSPEC-ENF-002",
    "CKSPEC-ENF-003",
    "CKSPEC-ENF-004",
    "CKSPEC-ENF-005",
    "CKSPEC-ENF-006",
    "CKSPEC-ENF-007",
    "CKSPEC-TEST-001",
    "CKSPEC-TEST-002",
    "CKSPEC-TEST-003",
    "CKSPEC-TEST-004",
    "CKSPEC-OUT-001",
    "CKSPEC-OUT-002",
    "CKSPEC-OUT-003",
    "CKSPEC-OUT-004",
    "CKSPEC-OUT-005",
    "CKSPEC-AGENT-001",
    "CKSPEC-AGENT-002",
    "CKSPEC-AGENT-003",
    "CKSPEC-AGENT-004",
    "CKSPEC-AGENT-005",
    "CKSPEC-CL-001",
    "CKSPEC-CL-002",
    "CKSPEC-CL-003",
    "CKSPEC-CL-004",
    "CKSPEC-CL-005",
    "CKSPEC-CL-006",
    "CKSPEC-CL-007",
];

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

    // ── ENF-005: Completeness check ─────────────────────────────
    let missing: Vec<&str> = EXPECTED_IDS
        .iter()
        .filter(|id| !mapping.requirements.contains_key(**id))
        .copied()
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

        // ENF-006: compile-time claims without violation tests
        if req.enforcement_level == "compile-time" && req.violation_tests.is_empty() {
            feedback.push(format!(
                "{req_id}: claims compile-time but has no violation test"
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
        implementation: "ckeletin-rust".to_string(),
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

/// Simple date without chrono dependency.
fn chrono_free_date() -> String {
    let output = Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .expect("date command failed");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
