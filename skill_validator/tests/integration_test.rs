use std::path::PathBuf;

use skill_validator::check;
use skill_validator::config::Config;
use skill_validator::query::{self, LintLevel, LintLevelOverrides};
use skill_validator::report;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_crates")
        .join(name)
}

fn run_lints_on(fixture: &str, filter_lints: &[&str]) -> check::LintReport {
    let root = fixture_path(fixture);
    let skills_dir = root.join("skills");
    let config = Config::default();
    let all_lints = query::load_builtin_lints();
    let overrides = LintLevelOverrides::default();
    let filter: Vec<String> = filter_lints.iter().map(|s| s.to_string()).collect();

    check::run_all_lints_with_root(
        &skills_dir,
        &root,
        &config,
        &all_lints,
        &overrides,
        &filter,
        false,
        None,
    )
    .expect("run_all_lints should not return an error")
}

fn snapshot_report(report: &check::LintReport) -> String {
    report::render_human(report, false)
}

// ---- Valid skill (no errors) ----

#[test]
fn valid_skill_has_no_errors() {
    let report = run_lints_on("valid_skill", &[]);
    assert_eq!(
        report.errors, 0,
        "expected no errors for valid_skill, got findings: {:?}",
        report
            .findings
            .iter()
            .filter(|f| f.level == LintLevel::Deny)
            .map(|f| format!("{}: {:?}", f.lint_id, f.detail))
            .collect::<Vec<_>>()
    );
    insta::assert_snapshot!("valid_skill", snapshot_report(&report));
}

// ---- Deny-level lints ----

#[test]
fn detects_missing_name() {
    let report = run_lints_on("missing_name", &["skill_missing_name"]);
    assert!(report.errors > 0, "expected errors for missing_name fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_missing_name"));
    insta::assert_snapshot!("missing_name", snapshot_report(&report));
}

#[test]
fn detects_missing_description() {
    let report = run_lints_on("missing_description", &["skill_missing_description"]);
    assert!(report.errors > 0, "expected errors for missing_description fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_missing_description"));
    insta::assert_snapshot!("missing_description", snapshot_report(&report));
}

#[test]
fn detects_flat_layout() {
    let report = run_lints_on("flat_layout", &["skill_flat_layout"]);
    assert!(report.errors > 0, "expected errors for flat_layout fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_flat_layout"));
    insta::assert_snapshot!("flat_layout", snapshot_report(&report));
}

#[test]
fn detects_missing_frontmatter() {
    let report = run_lints_on("missing_frontmatter", &["skill_missing_frontmatter"]);
    assert!(report.errors > 0, "expected errors for missing_frontmatter fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_missing_frontmatter"));
    insta::assert_snapshot!("missing_frontmatter", snapshot_report(&report));
}

#[test]
fn detects_duplicate_name() {
    let report = run_lints_on("duplicate_name", &["skill_duplicate_name"]);
    assert!(report.errors > 0, "expected errors for duplicate_name fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_duplicate_name"));
    insta::assert_snapshot!("duplicate_name", snapshot_report(&report));
}

#[test]
fn detects_mixed_languages() {
    let report = run_lints_on("mixed_languages", &["skill_mixed_script_languages"]);
    assert!(report.errors > 0, "expected errors for mixed_languages fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_mixed_script_languages"));
    insta::assert_snapshot!("mixed_languages", snapshot_report(&report));
}

#[test]
fn detects_name_mismatch() {
    let report = run_lints_on("name_mismatch", &["skill_name_mismatch"]);
    assert!(report.errors > 0, "expected errors for name_mismatch fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_name_mismatch"));
    insta::assert_snapshot!("name_mismatch", snapshot_report(&report));
}

#[test]
fn detects_name_invalid_format() {
    let report = run_lints_on("name_invalid_format", &["skill_name_invalid_format"]);
    assert!(report.errors > 0, "expected errors for name_invalid_format fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_name_invalid_format"));
    insta::assert_snapshot!("name_invalid_format", snapshot_report(&report));
}

#[test]
fn detects_name_consecutive_hyphens() {
    let report = run_lints_on("name_consecutive_hyphens", &["skill_name_consecutive_hyphens"]);
    assert!(report.errors > 0, "expected errors for name_consecutive_hyphens fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_name_consecutive_hyphens"));
    insta::assert_snapshot!("name_consecutive_hyphens", snapshot_report(&report));
}

#[test]
fn detects_name_too_long() {
    let report = run_lints_on("name_too_long", &["skill_name_too_long"]);
    assert!(report.errors > 0, "expected errors for name_too_long fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_name_too_long"));
    insta::assert_snapshot!("name_too_long", snapshot_report(&report));
}

#[test]
fn detects_description_too_long() {
    let report = run_lints_on("description_too_long", &["skill_description_too_long"]);
    assert!(report.errors > 0, "expected errors for description_too_long fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_description_too_long"));
    insta::assert_snapshot!("description_too_long", snapshot_report(&report));
}

// ---- Warn-level lints ----

#[test]
fn detects_description_too_short() {
    let report = run_lints_on("description_too_short", &["skill_description_too_short"]);
    assert!(report.warnings > 0, "expected warnings for description_too_short fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_description_too_short"));
    insta::assert_snapshot!("description_too_short", snapshot_report(&report));
}

#[test]
fn detects_body_too_long() {
    let report = run_lints_on("body_too_long", &["skill_body_too_long"]);
    assert!(report.warnings > 0, "expected warnings for body_too_long fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_body_too_long"));
    insta::assert_snapshot!("body_too_long", snapshot_report(&report));
}

#[test]
fn detects_missing_examples_section() {
    let report = run_lints_on("missing_examples", &["skill_missing_examples_section"]);
    assert!(report.warnings > 0, "expected warnings for missing_examples fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_missing_examples_section"));
    insta::assert_snapshot!("missing_examples", snapshot_report(&report));
}

#[test]
fn detects_missing_guidelines_section() {
    let report = run_lints_on("missing_guidelines", &["skill_missing_guidelines_section"]);
    assert!(report.warnings > 0, "expected warnings for missing_guidelines fixture");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_missing_guidelines_section"));
    insta::assert_snapshot!("missing_guidelines", snapshot_report(&report));
}

// ---- Allow-level lints (opt-in) ----

#[test]
fn allow_level_no_scripts_silent_by_default() {
    let report = run_lints_on("no_scripts", &["skill_has_no_scripts"]);
    assert_eq!(report.errors, 0, "Allow-level lint should not produce errors by default");
    assert_eq!(report.warnings, 0, "Allow-level lint should not produce warnings by default");
    assert!(report.findings.is_empty(), "Allow-level lint should produce no findings");
}

#[test]
fn allow_level_no_scripts_fires_when_promoted() {
    let root = fixture_path("no_scripts");
    let skills_dir = root.join("skills");
    let config = Config::default();
    let all_lints = query::load_builtin_lints();
    let overrides = query::LintLevelOverrides {
        warn: ["skill_has_no_scripts".to_string()].into_iter().collect(),
        ..query::LintLevelOverrides::default()
    };
    let report = check::run_all_lints_with_root(
        &skills_dir,
        &root,
        &config,
        &all_lints,
        &overrides,
        &[String::from("skill_has_no_scripts")],
        false,
        None,
    )
    .expect("should not return an error");
    assert!(report.warnings > 0, "no_scripts should warn when promoted via --warn");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_has_no_scripts"));
    insta::assert_snapshot!("no_scripts_promoted", snapshot_report(&report));
}

#[test]
fn allow_level_no_references_silent_by_default() {
    let report = run_lints_on("no_references", &["skill_has_no_references"]);
    assert_eq!(report.errors, 0, "Allow-level lint should not produce errors by default");
    assert_eq!(report.warnings, 0, "Allow-level lint should not produce warnings by default");
    assert!(report.findings.is_empty(), "Allow-level lint should produce no findings");
}

#[test]
fn allow_level_no_references_fires_when_promoted() {
    let root = fixture_path("no_references");
    let skills_dir = root.join("skills");
    let config = Config::default();
    let all_lints = query::load_builtin_lints();
    let overrides = query::LintLevelOverrides {
        warn: ["skill_has_no_references".to_string()].into_iter().collect(),
        ..query::LintLevelOverrides::default()
    };
    let report = check::run_all_lints_with_root(
        &skills_dir,
        &root,
        &config,
        &all_lints,
        &overrides,
        &[String::from("skill_has_no_references")],
        false,
        None,
    )
    .expect("should not return an error");
    assert!(report.warnings > 0, "no_references should warn when promoted via --warn");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_has_no_references"));
    insta::assert_snapshot!("no_references_promoted", snapshot_report(&report));
}
