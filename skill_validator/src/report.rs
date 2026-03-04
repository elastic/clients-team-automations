use crate::check::{LintFinding, LintReport};
use crate::query::LintLevel;
use std::fmt;

use miette::{LabeledSpan, NamedSource, SourceSpan};

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
        OutputFormat::Human => print_human_rich(report, verbose),
        OutputFormat::Json => print_json(report),
        OutputFormat::GithubActions => print_github_actions(report),
    }
}

/// Plain-text rendering (used by snapshot tests via lib.rs and as fallback).
#[allow(dead_code)]
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

// ---------------------------------------------------------------------------
// Rich diagnostic output (miette)
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
#[error("{detail}")]
struct RichDiagnostic {
    detail: String,
    src: NamedSource<String>,
    span: SourceSpan,
    label: String,
    severity: miette::Severity,
    code: String,
    help: Option<String>,
}

impl miette::Diagnostic for RichDiagnostic {
    fn severity(&self) -> Option<miette::Severity> {
        Some(self.severity)
    }

    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new(&self.code))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.help
            .as_ref()
            .map(|h| Box::new(h.as_str()) as Box<dyn fmt::Display>)
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.src)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        Some(Box::new(std::iter::once(LabeledSpan::new(
            Some(self.label.clone()),
            self.span.offset(),
            self.span.len(),
        ))))
    }
}

fn print_human_rich(report: &LintReport, verbose: bool) {
    let handler = miette::GraphicalReportHandler::new_themed(miette::GraphicalTheme::unicode());

    for finding in &report.findings {
        match try_render_rich(finding, verbose, &handler) {
            Some(rendered) => eprint!("{rendered}"),
            None => eprint!("{}", render_finding_plain(finding, verbose)),
        }
    }

    eprintln!();
    eprintln!(
        "Ran {} lints: {} errors, {} warnings",
        report.lints_run, report.errors, report.warnings
    );
}

fn try_render_rich(
    finding: &LintFinding,
    verbose: bool,
    handler: &miette::GraphicalReportHandler,
) -> Option<String> {
    let filename = finding.filename.as_ref()?;
    let line = finding.line? as usize;

    let source = std::fs::read_to_string(filename).ok()?;

    let (byte_start, line_len) = byte_offset_for_line(&source, line)?;
    let span_len = if let Some(end) = finding.end_line {
        let end = end as usize;
        if end > line {
            let (end_offset, end_len) = byte_offset_for_line(&source, end)?;
            (end_offset + end_len) - byte_start
        } else {
            line_len
        }
    } else {
        line_len
    };

    let severity = match finding.level {
        LintLevel::Deny => miette::Severity::Error,
        LintLevel::Warn => miette::Severity::Warning,
        LintLevel::Allow => return None,
    };

    let detail = finding
        .detail
        .clone()
        .unwrap_or_else(|| finding.message.clone());

    let help = if verbose {
        Some(finding.message.clone())
    } else {
        None
    };

    let diag = RichDiagnostic {
        detail,
        src: NamedSource::new(filename, source),
        span: SourceSpan::new(byte_start.into(), span_len),
        label: finding.lint_id.clone(),
        severity,
        code: finding.lint_id.clone(),
        help,
    };

    let mut buf = String::new();
    handler.render_report(&mut buf, &diag).ok()?;
    Some(buf)
}

fn byte_offset_for_line(source: &str, line_number: usize) -> Option<(usize, usize)> {
    let mut offset = 0;
    for (i, line) in source.lines().enumerate() {
        if i + 1 == line_number {
            return Some((offset, line.len().max(1)));
        }
        offset += line.len() + 1;
    }
    None
}

fn render_finding_plain(finding: &LintFinding, verbose: bool) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    let level_str = match finding.level {
        LintLevel::Deny => "error",
        LintLevel::Warn => "warning",
        LintLevel::Allow => return out,
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

        let msg = finding.detail.as_deref().unwrap_or(&finding.message);

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
