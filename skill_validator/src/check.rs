use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use trustfall::{FieldValue, TransparentValue, execute_query};

use crate::adapter::SkillsAdapter;
use crate::config::Config;
use crate::data;
use crate::query::{LintLevel, LintLevelOverrides, SkillLint};
use crate::schema;

#[derive(Debug, Clone)]
pub struct LintFinding {
    pub lint_id: String,
    pub level: LintLevel,
    pub message: String,
    pub detail: Option<String>,
    pub filename: Option<String>,
    pub line: Option<i64>,
}

#[derive(Debug)]
pub struct LintReport {
    pub findings: Vec<LintFinding>,
    pub lints_run: usize,
    pub errors: usize,
    pub warnings: usize,
}

impl LintReport {
    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }
}

pub fn run_all_lints(
    skills_dir: &Path,
    config: &Config,
    builtin_lints: &[SkillLint],
    overrides: &LintLevelOverrides,
    filter_ids: &[String],
    quiet: bool,
) -> Result<LintReport, String> {
    let repo_root = std::env::current_dir().map_err(|e| format!("cannot get cwd: {e}"))?;
    run_all_lints_with_root(skills_dir, &repo_root, config, builtin_lints, overrides, filter_ids, quiet)
}

pub fn run_all_lints_with_root(
    skills_dir: &Path,
    repo_root: &Path,
    config: &Config,
    builtin_lints: &[SkillLint],
    overrides: &LintLevelOverrides,
    filter_ids: &[String],
    quiet: bool,
) -> Result<LintReport, String> {

    let skills_data = data::load_skills_data(skills_dir, repo_root, config);
    let adapter = SkillsAdapter::new(skills_data);
    let schema = schema::schema();

    let custom_lints = crate::query::load_custom_lints(&config.custom_lint_dirs);
    let mut all_lints: Vec<&SkillLint> = builtin_lints.iter().collect();
    all_lints.extend(custom_lints.iter());

    // Filter to requested lints if specified
    if !filter_ids.is_empty() {
        all_lints.retain(|l| filter_ids.contains(&l.id));
    }

    let mut findings = Vec::new();
    let mut errors = 0usize;
    let mut warnings = 0usize;
    let lints_run = all_lints.len();

    for lint in &all_lints {
        let level = lint.effective_level(&config.lints, overrides);
        if level == LintLevel::Allow {
            continue;
        }
        if quiet && level == LintLevel::Warn {
            continue;
        }

        let args: BTreeMap<String, FieldValue> = lint
            .arguments
            .iter()
            .map(|(k, v)| (k.clone(), transparent_to_field(v)))
            .collect();

        let results = execute_query(&schema, Arc::new(adapter.clone()), &lint.query, args);
        let results = match results {
            Ok(r) => r,
            Err(e) => {
                findings.push(LintFinding {
                    lint_id: lint.id.clone(),
                    level: LintLevel::Deny,
                    message: format!("Lint query failed: {e}"),
                    detail: None,
                    filename: None,
                    line: None,
                });
                errors += 1;
                continue;
            }
        };

        let result_rows: Vec<BTreeMap<Arc<str>, FieldValue>> = results.collect();

        if result_rows.is_empty() {
            continue;
        }

        for row in &result_rows {
            let detail = lint
                .per_result_error_template
                .as_ref()
                .map(|tmpl| render_template(tmpl, row));

            let filename = find_field_string(row, &["span_filename", "filename"]);
            let line = find_field_i64(row, &["span_begin_line", "begin_line"]);

            findings.push(LintFinding {
                lint_id: lint.id.clone(),
                level: level.clone(),
                message: lint.error_message.clone(),
                detail,
                filename,
                line,
            });

            match level {
                LintLevel::Deny => errors += 1,
                LintLevel::Warn => warnings += 1,
                LintLevel::Allow => {}
            }
        }
    }

    Ok(LintReport {
        findings,
        lints_run,
        errors,
        warnings,
    })
}

fn transparent_to_field(tv: &TransparentValue) -> FieldValue {
    match tv {
        TransparentValue::String(s) => FieldValue::from(&**s),
        TransparentValue::Float64(f) => FieldValue::Float64(*f),
        TransparentValue::Int64(i) => FieldValue::Int64(*i),
        TransparentValue::Uint64(u) => FieldValue::Uint64(*u),
        TransparentValue::Boolean(b) => FieldValue::Boolean(*b),
        TransparentValue::Null => FieldValue::Null,
        TransparentValue::List(l) => {
            let items: Vec<FieldValue> = l.iter().map(transparent_to_field).collect();
            FieldValue::List(items.into())
        }
        _ => FieldValue::Null,
    }
}

fn find_field_string(row: &BTreeMap<Arc<str>, FieldValue>, candidates: &[&str]) -> Option<String> {
    for key in candidates {
        let arc_key: Arc<str> = Arc::from(*key);
        if let Some(FieldValue::String(s)) = row.get(&arc_key) {
            return Some(s.to_string());
        }
    }
    None
}

fn find_field_i64(row: &BTreeMap<Arc<str>, FieldValue>, candidates: &[&str]) -> Option<i64> {
    for key in candidates {
        let arc_key: Arc<str> = Arc::from(*key);
        match row.get(&arc_key) {
            Some(FieldValue::Int64(i)) => return Some(*i),
            Some(FieldValue::Uint64(u)) => return Some(*u as i64),
            _ => {}
        }
    }
    None
}

fn render_template(template: &str, row: &BTreeMap<Arc<str>, FieldValue>) -> String {
    let mut result = template.to_string();
    for (key, value) in row {
        let placeholder = format!("{{{{{key}}}}}");
        let replacement = match value {
            FieldValue::String(s) => s.to_string(),
            FieldValue::Int64(i) => i.to_string(),
            FieldValue::Uint64(u) => u.to_string(),
            FieldValue::Float64(f) => f.to_string(),
            FieldValue::Boolean(b) => b.to_string(),
            FieldValue::Null => "<null>".to_string(),
            FieldValue::List(l) => format!("{l:?}"),
            _ => format!("{value:?}"),
        };
        result = result.replace(&placeholder, &replacement);
    }
    result
}
