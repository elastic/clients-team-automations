use std::path::PathBuf;

use skill_validator::check;
use skill_validator::config::Config;
use skill_validator::query::{self, LintLevel, LintLevelOverrides};

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
    )
    .expect("run_all_lints should not return an error")
}

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
}

#[test]
fn detects_missing_name() {
    let report = run_lints_on("missing_name", &["skill_missing_name"]);
    assert!(
        report.errors > 0,
        "expected errors for missing_name fixture"
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.lint_id == "skill_missing_name"),
        "expected skill_missing_name finding"
    );
}

#[test]
fn detects_missing_description() {
    let report = run_lints_on("missing_description", &["skill_missing_description"]);
    assert!(
        report.errors > 0,
        "expected errors for missing_description fixture"
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.lint_id == "skill_missing_description"),
        "expected skill_missing_description finding"
    );
}

#[test]
fn detects_flat_layout() {
    let report = run_lints_on("flat_layout", &["skill_flat_layout"]);
    assert!(
        report.errors > 0,
        "expected errors for flat_layout fixture"
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.lint_id == "skill_flat_layout"),
        "expected skill_flat_layout finding"
    );
}

#[test]
fn detects_missing_frontmatter() {
    let report = run_lints_on("missing_frontmatter", &["skill_missing_frontmatter"]);
    assert!(
        report.errors > 0,
        "expected errors for missing_frontmatter fixture"
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.lint_id == "skill_missing_frontmatter"),
        "expected skill_missing_frontmatter finding"
    );
}

#[test]
fn detects_duplicate_name() {
    let report = run_lints_on("duplicate_name", &["skill_duplicate_name"]);
    assert!(
        report.errors > 0,
        "expected errors for duplicate_name fixture"
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.lint_id == "skill_duplicate_name"),
        "expected skill_duplicate_name finding"
    );
}

#[test]
fn detects_mixed_languages() {
    let report = run_lints_on("mixed_languages", &["skill_mixed_script_languages"]);
    assert!(
        report.errors > 0,
        "expected errors for mixed_languages fixture, findings: {:?}",
        report
            .findings
            .iter()
            .map(|f| &f.lint_id)
            .collect::<Vec<_>>()
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.lint_id == "skill_mixed_script_languages"),
        "expected skill_mixed_script_languages finding"
    );
}
