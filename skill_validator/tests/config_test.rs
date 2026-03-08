use std::collections::BTreeMap;
use std::path::PathBuf;

use skill_validator::check;
use skill_validator::config::Config;
use skill_validator::query::{self, LintLevel, LintLevelOverrides};

fn builtin_lint_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lints")
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_crates")
        .join(name)
}

fn run_lints_with_config(
    fixture: &str,
    config: Config,
    filter_lints: &[&str],
) -> check::LintReport {
    let root = fixture_path(fixture);
    let skills_dir = root.join("skills");
    let all_lints = query::load_lints(&[builtin_lint_dir()]);
    let overrides = LintLevelOverrides::default();
    let filter: Vec<String> = filter_lints.iter().map(|s| s.to_string()).collect();

    check::run_all_lints_with_root(
        &skills_dir,
        &root,
        &config,
        &all_lints,
        &overrides,
        &check::LintRunOptions {
            filter_ids: &filter,
            ..check::LintRunOptions::default()
        },
    )
    .expect("run_all_lints should not return an error")
}

fn run_lints_with_overrides(
    fixture: &str,
    config: Config,
    overrides: LintLevelOverrides,
    filter_lints: &[&str],
) -> check::LintReport {
    let root = fixture_path(fixture);
    let skills_dir = root.join("skills");
    let all_lints = query::load_lints(&[builtin_lint_dir()]);
    let filter: Vec<String> = filter_lints.iter().map(|s| s.to_string()).collect();

    check::run_all_lints_with_root(
        &skills_dir,
        &root,
        &config,
        &all_lints,
        &overrides,
        &check::LintRunOptions {
            filter_ids: &filter,
            ..check::LintRunOptions::default()
        },
    )
    .expect("run_all_lints should not return an error")
}

#[test]
fn skill_only_name_passes_suffix_lint() {
    let report = run_lints_with_config(
        "custom_name_pattern",
        Config::default(),
        &["skill_name_missing_folder_suffix"],
    );
    assert_eq!(
        report.errors, 0,
        "name 'my-skill' ends with folder 'my-skill', suffix lint should not fire"
    );
}

#[test]
fn skill_only_name_fails_prefix_lint() {
    let report = run_lints_with_config(
        "custom_name_pattern",
        Config::default(),
        &["skill_name_missing_group_prefix"],
    );
    assert!(
        report.errors > 0,
        "name 'my-skill' does not start with group 'test', prefix lint should fire"
    );
}

#[test]
fn config_lint_level_override_promotes_warn_to_deny() {
    let mut lints = BTreeMap::new();
    lints.insert("skill_body_too_long".to_string(), LintLevel::Deny);
    let config = Config {
        lints,
        ..Config::default()
    };
    let report = run_lints_with_config("body_too_long", config, &["skill_body_too_long"]);
    assert!(
        report.errors > 0,
        "body_too_long should be an error when promoted to Deny via config"
    );
    assert!(report.findings.iter().all(|f| f.level == LintLevel::Deny));
}

#[test]
fn config_lint_level_override_suppresses_with_allow() {
    let mut lints = BTreeMap::new();
    lints.insert(
        "skill_missing_examples_section".to_string(),
        LintLevel::Allow,
    );
    let config = Config {
        lints,
        ..Config::default()
    };
    let report = run_lints_with_config(
        "missing_examples",
        config,
        &["skill_missing_examples_section"],
    );
    assert_eq!(
        report.warnings, 0,
        "missing_examples_section should be suppressed when set to Allow"
    );
    assert!(report.findings.is_empty());
}

#[test]
fn cli_override_takes_precedence_over_config() {
    let mut config_lints = BTreeMap::new();
    config_lints.insert("skill_body_too_long".to_string(), LintLevel::Allow);
    let config = Config {
        lints: config_lints,
        ..Config::default()
    };

    let overrides = LintLevelOverrides {
        deny: ["skill_body_too_long".to_string()].into_iter().collect(),
        ..LintLevelOverrides::default()
    };

    let report = run_lints_with_overrides(
        "body_too_long",
        config,
        overrides,
        &["skill_body_too_long"],
    );
    assert!(
        report.errors > 0,
        "CLI --deny should override config Allow"
    );
}

#[test]
fn custom_data_extensions_changes_classification() {
    let config = Config {
        data_extensions: vec!["py".to_string(), "sh".to_string()],
        ..Config::default()
    };
    let report = run_lints_with_config(
        "mixed_languages",
        config,
        &["skill_mixed_script_languages"],
    );
    assert_eq!(
        report.errors, 0,
        "when .py and .sh are classified as data files, mixed_script_languages should not fire"
    );
}

#[test]
fn custom_lint_dirs_loads_and_runs_custom_lints() {
    let root = fixture_path("custom_lints");
    let skills_dir = root.join("skills");
    let config = Config::default();
    let all_lints = query::load_lints(&[root.join("my-lints")]);
    let overrides = LintLevelOverrides::default();

    let filter = vec![String::from("skill_must_have_license")];
    let report = check::run_all_lints_with_root(
        &skills_dir,
        &root,
        &config,
        &all_lints,
        &overrides,
        &check::LintRunOptions {
            filter_ids: &filter,
            ..check::LintRunOptions::default()
        },
    )
    .expect("should not return an error");

    assert_eq!(report.findings.len(), 0,
        "skill has a license, so the custom lint should not fire");
}

#[test]
fn custom_lint_dirs_fires_on_missing_license() {
    let root = fixture_path("missing_name");
    let skills_dir = root.join("skills");
    let custom_lint_dir = fixture_path("custom_lints").join("my-lints");
    let config = Config::default();
    let all_lints = query::load_lints(&[custom_lint_dir]);
    let overrides = LintLevelOverrides::default();

    let filter = vec![String::from("skill_must_have_license")];
    let report = check::run_all_lints_with_root(
        &skills_dir,
        &root,
        &config,
        &all_lints,
        &overrides,
        &check::LintRunOptions {
            filter_ids: &filter,
            ..check::LintRunOptions::default()
        },
    )
    .expect("should not return an error");

    assert!(report.warnings > 0,
        "missing_name fixture has no license, custom lint should fire");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_must_have_license"));
}

#[test]
fn missing_config_file_uses_defaults() {
    let config = Config::load(std::path::Path::new("nonexistent.toml"));
    assert_eq!(config.skills_dir, PathBuf::from("skills"));
    assert!(config.lints.is_empty());
    assert!(config.custom_lint_dirs.is_empty());
}

#[test]
fn config_load_from_valid_toml() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let toml_path = dir.path().join(".skill-validator.toml");
    std::fs::write(
        &toml_path,
        r#"
skills_dir = "my-skills"

[lints]
skill_body_too_long = "Deny"
"#,
    )
    .expect("write toml");

    let config = Config::load(&toml_path);
    assert_eq!(config.skills_dir, PathBuf::from("my-skills"));
    assert_eq!(
        config.lints.get("skill_body_too_long"),
        Some(&LintLevel::Deny)
    );
}

#[test]
fn duplicate_lint_id_returns_error() {
    let root = fixture_path("custom_lints_collision");
    let skills_dir = root.join("skills");
    let config = Config::default();
    let all_lints = query::load_lints(&[builtin_lint_dir(), root.join("collision-lints")]);
    let overrides = LintLevelOverrides::default();

    let result = check::run_all_lints_with_root(
        &skills_dir,
        &root,
        &config,
        &all_lints,
        &overrides,
        &check::LintRunOptions::default(),
    );

    let err = result.expect_err("expected an error due to duplicate lint id");
    let msg = err.to_string();
    assert!(
        msg.contains("duplicate lint id"),
        "error message should mention duplicate lint ids, got: {msg}"
    );
    assert!(
        msg.contains("skill_missing_name"),
        "error message should name the colliding id, got: {msg}"
    );
}
