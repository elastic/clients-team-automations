use crate::check::LintReport;
use crate::query::LintLevel;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    Human,
    Json,
    GithubActions,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Human => write!(f, "human"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::GithubActions => write!(f, "github-actions"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "human" => Ok(OutputFormat::Human),
            "json" => Ok(OutputFormat::Json),
            "github-actions" => Ok(OutputFormat::GithubActions),
            _ => Err(format!(
                "unknown format '{s}', expected: human, json, github-actions"
            )),
        }
    }
}

pub fn print_report(report: &LintReport, format: &OutputFormat, verbose: bool) {
    match format {
        OutputFormat::Human => eprint!("{}", render_human(report, verbose)),
        OutputFormat::Json => print_json(report),
        OutputFormat::GithubActions => print_github_actions(report),
    }
}

pub fn render_human(report: &LintReport, verbose: bool) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    for finding in &report.findings {
        let level_str = match finding.level {
            LintLevel::Deny => "error",
            LintLevel::Warn => "warning",
            LintLevel::Allow => continue,
        };

        let location = match (&finding.filename, finding.line) {
            (Some(f), Some(l)) => format!("{f}:{l}"),
            (Some(f), None) => f.clone(),
            _ => String::new(),
        };

        if let Some(detail) = &finding.detail {
            if !location.is_empty() {
                writeln!(out, "{level_str}[{}]: {location}: {detail}", finding.lint_id).unwrap();
            } else {
                writeln!(out, "{level_str}[{}]: {detail}", finding.lint_id).unwrap();
            }
        } else if !location.is_empty() {
            writeln!(
                out,
                "{level_str}[{}]: {location}: {}",
                finding.lint_id, finding.message
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "{level_str}[{}]: {}",
                finding.lint_id, finding.message
            )
            .unwrap();
        }

        if verbose {
            writeln!(out, "  help: {}", finding.message).unwrap();
        }
    }

    writeln!(out).unwrap();
    writeln!(
        out,
        "Ran {} lints: {} errors, {} warnings",
        report.lints_run, report.errors, report.warnings
    )
    .unwrap();

    out
}

fn print_json(report: &LintReport) {
    #[derive(serde::Serialize)]
    struct JsonReport {
        lints_run: usize,
        errors: usize,
        warnings: usize,
        findings: Vec<JsonFinding>,
    }

    #[derive(serde::Serialize)]
    struct JsonFinding {
        lint_id: String,
        level: String,
        message: String,
        detail: Option<String>,
        filename: Option<String>,
        line: Option<i64>,
    }

    let json_report = JsonReport {
        lints_run: report.lints_run,
        errors: report.errors,
        warnings: report.warnings,
        findings: report
            .findings
            .iter()
            .map(|f| JsonFinding {
                lint_id: f.lint_id.clone(),
                level: match f.level {
                    LintLevel::Deny => "error".to_string(),
                    LintLevel::Warn => "warning".to_string(),
                    LintLevel::Allow => "allow".to_string(),
                },
                message: f.message.clone(),
                detail: f.detail.clone(),
                filename: f.filename.clone(),
                line: f.line,
            })
            .collect(),
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&json_report).expect("failed to serialize report")
    );
}

fn print_github_actions(report: &LintReport) {
    for finding in &report.findings {
        let cmd = match finding.level {
            LintLevel::Deny => "error",
            LintLevel::Warn => "warning",
            LintLevel::Allow => continue,
        };

        let msg = finding
            .detail
            .as_deref()
            .unwrap_or(&finding.message);

        match (&finding.filename, finding.line) {
            (Some(file), Some(line)) => {
                println!("::{cmd} file={file},line={line}::[{}] {msg}", finding.lint_id);
            }
            (Some(file), None) => {
                println!("::{cmd} file={file}::[{}] {msg}", finding.lint_id);
            }
            _ => {
                println!("::{cmd}::[{}] {msg}", finding.lint_id);
            }
        }
    }
}
