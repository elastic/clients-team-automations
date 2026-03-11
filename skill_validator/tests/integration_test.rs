use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use skill_validator::adapter::SkillsAdapter;
use skill_validator::check;
use skill_validator::config::Config;
use skill_validator::data;
use skill_validator::query::{self, LintLevel, LintLevelOverrides};
use skill_validator::report;
use skill_validator::schema;
use trustfall::{execute_query, FieldValue};

fn builtin_lint_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lints")
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_crates")
        .join(name)
}

fn run_lints_on(fixture: &str, filter_lints: &[&str]) -> check::LintReport {
    let root = fixture_path(fixture);
    let skills_dir = root.join("skills");
    let config = Config::default();
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
fn detects_name_missing_group_prefix() {
    let report = run_lints_on("name_mismatch", &["skill_name_missing_group_prefix"]);
    assert!(report.errors > 0, "expected errors for name_mismatch fixture (group prefix)");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_name_missing_group_prefix"));
    insta::assert_snapshot!("name_missing_group_prefix", snapshot_report(&report));
}

#[test]
fn detects_name_missing_folder_suffix() {
    let report = run_lints_on("name_mismatch", &["skill_name_missing_folder_suffix"]);
    assert!(report.errors > 0, "expected errors for name_mismatch fixture (folder suffix)");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_name_missing_folder_suffix"));
    insta::assert_snapshot!("name_missing_folder_suffix", snapshot_report(&report));
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
    let all_lints = query::load_lints(&[builtin_lint_dir()]);
    let overrides = query::LintLevelOverrides {
        warn: ["skill_has_no_scripts".to_string()].into_iter().collect(),
        ..query::LintLevelOverrides::default()
    };
    let filter = vec![String::from("skill_has_no_scripts")];
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
    let all_lints = query::load_lints(&[builtin_lint_dir()]);
    let overrides = query::LintLevelOverrides {
        warn: ["skill_has_no_references".to_string()].into_iter().collect(),
        ..query::LintLevelOverrides::default()
    };
    let filter = vec![String::from("skill_has_no_references")];
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
    assert!(report.warnings > 0, "no_references should warn when promoted via --warn");
    assert!(report.findings.iter().any(|f| f.lint_id == "skill_has_no_references"));
    insta::assert_snapshot!("no_references_promoted", snapshot_report(&report));
}

// ---- Scope filtering for DiscoveredDirectory ----

/// Regression test: with `scope: changed`, directories that were NOT in the
/// changed set should not appear in `discovered_dirs` and therefore should not
/// trigger `skill_directory_no_skill_md` (or any other DiscoveredDirectory lint).
#[test]
fn scope_filter_excludes_unchanged_directories_from_discovered_dirs() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let root = tmp.path();
    let skills_dir = root.join("skills");

    // Two directories at skill depth, neither has a SKILL.md.
    // Only "in-scope" is in the changed set; "out-of-scope" is not.
    for dir in &["group/in-scope", "group/out-of-scope"] {
        let d = skills_dir.join(dir);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("helper.md"), "# helper").unwrap();
    }

    let config = Config::default();
    let scope: std::collections::HashSet<String> =
        ["skills/group/in-scope".to_string()].into_iter().collect();

    let data = data::load_skills_data(&skills_dir, root, &config, Some(&scope));

    let dir_paths: Vec<&str> = data
        .discovered_dirs
        .iter()
        .map(|d| d.path.as_str())
        .collect();

    assert!(
        dir_paths.iter().any(|p| *p == "skills/group/in-scope"),
        "in-scope directory should be present"
    );
    assert!(
        !dir_paths.iter().any(|p| *p == "skills/group/out-of-scope"),
        "out-of-scope directory should be excluded; found: {dir_paths:?}"
    );
}

// ---- Referenced paths (query-based) ----

fn query_fixture(fixture: &str, query: &str) -> Vec<BTreeMap<Arc<str>, FieldValue>> {
    let root = fixture_path(fixture);
    let skills_dir = root.join("skills");
    let config = Config::default();
    let skills_data = data::load_skills_data(&skills_dir, &root, &config, None);
    let adapter = SkillsAdapter::new(skills_data);
    let schema = schema::schema();
    let args: BTreeMap<String, FieldValue> = BTreeMap::new();
    execute_query(&schema, Arc::new(adapter), query, args)
        .expect("query should succeed")
        .collect()
}

#[test]
fn skill_referenced_paths_from_markdown() {
    let rows = query_fixture(
        "cross_references",
        r#"{
            Skill {
                skill_file_path @output
                referenced_path {
                    raw_path @output
                    resolved_path @output
                    kind @output
                    line_number @output
                }
            }
        }"#,
    );
    assert!(
        !rows.is_empty(),
        "expected referenced_path results from SKILL.md links"
    );
    let raw_paths: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            let key: Arc<str> = Arc::from("raw_path");
            match r.get(&key)? {
                FieldValue::String(s) => Some(s.to_string()),
                _ => None,
            }
        })
        .collect();
    assert!(
        raw_paths.contains(&"../../shared/shared-skill/guide.md".to_string()),
        "should find the cross-skill markdown link"
    );
    assert!(
        raw_paths.contains(&"../../shared/shared-skill/assets/diagram.png".to_string()),
        "should find the cross-skill image link"
    );
    assert!(
        raw_paths.contains(&"./scripts/helper.js".to_string()),
        "should find the local-skill markdown link"
    );
    assert!(
        !raw_paths.iter().any(|p| p.starts_with("https://")),
        "should not include https URLs"
    );
}

#[test]
fn subdir_file_referenced_paths_from_js() {
    let rows = query_fixture(
        "cross_references",
        r#"{
            Skill {
                sub_dir {
                    file {
                        name @output(name: "file_name")
                        referenced_path {
                            raw_path @output
                            resolved_path @output
                            kind @output
                        }
                    }
                }
            }
        }"#,
    );
    assert!(
        !rows.is_empty(),
        "expected referenced_path results from JS imports"
    );
    let import_rows: Vec<_> = rows
        .iter()
        .filter(|r| {
            let key: Arc<str> = Arc::from("kind");
            matches!(r.get(&key), Some(FieldValue::String(s)) if s.as_ref() == "js_import")
        })
        .collect();
    assert!(
        import_rows.len() >= 2,
        "should have at least 2 js_import references, got {}",
        import_rows.len()
    );
}

#[test]
fn subdir_file_content_readable() {
    let rows = query_fixture(
        "cross_references",
        r#"{
            Skill {
                sub_dir {
                    file {
                        name @output(name: "file_name")
                        content @output
                    }
                }
            }
        }"#,
    );
    assert!(!rows.is_empty(), "should have files with content");
    let has_content = rows.iter().any(|r| {
        let key: Arc<str> = Arc::from("content");
        matches!(r.get(&key), Some(FieldValue::String(s)) if !s.is_empty())
    });
    assert!(has_content, "at least one file should have non-empty content");
}

// ---- Line precision (span field accuracy) ----

#[test]
fn name_too_long_finding_points_to_name_field() {
    let report = run_lints_on("line_precision", &["skill_name_too_long"]);
    assert!(report.errors > 0, "expected errors for line_precision fixture (name too long)");
    let finding = report
        .findings
        .iter()
        .find(|f| f.lint_id == "skill_name_too_long")
        .expect("skill_name_too_long finding not found");
    assert_eq!(
        finding.line,
        Some(2),
        "name_too_long finding should point to line 2 (the name: field), got {:?}",
        finding.line
    );
}

#[test]
fn description_too_short_finding_points_to_description_field() {
    let report = run_lints_on("line_precision", &["skill_description_too_short"]);
    assert!(report.warnings > 0, "expected warnings for line_precision fixture (description too short)");
    let finding = report
        .findings
        .iter()
        .find(|f| f.lint_id == "skill_description_too_short")
        .expect("skill_description_too_short finding not found");
    assert_eq!(
        finding.line,
        Some(3),
        "description_too_short finding should point to line 3 (the description: field), got {:?}",
        finding.line
    );
}

#[test]
fn missing_name_finding_points_to_frontmatter_end() {
    let report = run_lints_on("missing_name", &["skill_missing_name"]);
    assert!(report.errors > 0, "expected errors for missing_name fixture");
    let finding = report
        .findings
        .iter()
        .find(|f| f.lint_id == "skill_missing_name")
        .expect("skill_missing_name finding not found");
    // The missing_name fixture has frontmatter closing --- on line 3
    assert_eq!(
        finding.line,
        Some(3),
        "missing_name finding should point to the closing --- line (3), got {:?}",
        finding.line
    );
}
