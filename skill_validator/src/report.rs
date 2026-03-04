use crate::check::{LintFinding, LintReport};
use crate::query::LintLevel;
use std::fmt;
use std::path::Path;

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

/// Plain-text rendering used by integration/snapshot tests via the lib crate.
/// Dead from the binary's perspective since main.rs re-declares modules privately.
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

const MAX_SNIPPET_LINES: usize = 8;

fn try_render_rich(
    finding: &LintFinding,
    verbose: bool,
    handler: &miette::GraphicalReportHandler,
) -> Option<String> {
    let filename = finding.filename.as_ref()?;
    let line = finding.line? as usize;

    let source = std::fs::read_to_string(filename).ok()?;

    let (byte_start, line_len) = byte_offset_for_line(&source, line)?;
    let original_end_line = finding.end_line.map(|e| e as usize).unwrap_or(line);
    let total_span_lines = if original_end_line > line {
        original_end_line - line + 1
    } else {
        1
    };

    let (span_len, truncated) = if total_span_lines > MAX_SNIPPET_LINES {
        let capped_end = line + MAX_SNIPPET_LINES - 1;
        let (end_offset, end_len) = byte_offset_for_line(&source, capped_end)?;
        ((end_offset + end_len) - byte_start, true)
    } else if let Some(end) = finding.end_line {
        let end = end as usize;
        if end > line {
            let (end_offset, end_len) = byte_offset_for_line(&source, end)?;
            ((end_offset + end_len) - byte_start, false)
        } else {
            (line_len, false)
        }
    } else {
        (line_len, false)
    };

    let label = if truncated {
        format!("{} ({} lines total)", finding.lint_id, total_span_lines)
    } else {
        finding.lint_id.clone()
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
        label,
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

#[derive(serde::Serialize)]
struct JsonReport {
    lints_run: usize,
    skills_checked: usize,
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

fn render_json(report: &LintReport) -> String {
    let json_report = JsonReport {
        lints_run: report.lints_run,
        skills_checked: report.skills_checked,
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

    serde_json::to_string_pretty(&json_report).expect("failed to serialize report")
}

fn print_json(report: &LintReport) {
    println!("{}", render_json(report));
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

pub fn write_json_file(report: &LintReport, path: &Path) -> Result<(), String> {
    let json = render_json(report);
    fs_err::write(path, json).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// GitHub Summary & PR comment
// ---------------------------------------------------------------------------

const COMMENT_MARKER: &str = "<!-- skill-validator-bot -->";

fn render_findings_table(findings: &[&LintFinding]) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    writeln!(out, "| Lint | File | Line | Detail |").unwrap();
    writeln!(out, "|------|------|------|--------|").unwrap();

    for f in findings {
        let detail = f
            .detail
            .as_deref()
            .unwrap_or(&f.message);
        let file = f
            .filename
            .as_deref()
            .unwrap_or("-");
        let line = f
            .line
            .map(|l| l.to_string())
            .unwrap_or_else(|| "-".to_string());
        writeln!(out, "| `{lint}` | `{file}` | {line} | {detail} |",
            lint = f.lint_id,
        ).unwrap();
    }

    out
}

fn render_summary_body(report: &LintReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    let status = if report.errors > 0 {
        "Failed"
    } else if report.warnings > 0 {
        "Passed with warnings"
    } else {
        "Passed"
    };

    let icon = if report.errors > 0 {
        "x"
    } else if report.warnings > 0 {
        "warning"
    } else {
        "heavy_check_mark"
    };

    writeln!(out, "## :{icon}: Skill Validator — {status}\n").unwrap();
    writeln!(
        out,
        "**{lints}** lints ran against **{skills}** skills — **{errors}** error(s), **{warnings}** warning(s)\n",
        lints = report.lints_run,
        skills = report.skills_checked,
        errors = report.errors,
        warnings = report.warnings,
    ).unwrap();

    let errors: Vec<&LintFinding> = report
        .findings
        .iter()
        .filter(|f| f.level == LintLevel::Deny)
        .collect();

    let warnings: Vec<&LintFinding> = report
        .findings
        .iter()
        .filter(|f| f.level == LintLevel::Warn)
        .collect();

    if !errors.is_empty() {
        writeln!(out, "### Errors\n").unwrap();
        out.push_str(&render_findings_table(&errors));
        out.push('\n');
    }

    if !warnings.is_empty() {
        writeln!(out, "<details>\n<summary>Warnings ({count})</summary>\n",
            count = warnings.len(),
        ).unwrap();
        out.push_str(&render_findings_table(&warnings));
        writeln!(out, "\n</details>\n").unwrap();
    }

    if errors.is_empty() && warnings.is_empty() {
        writeln!(out, "All checks passed.\n").unwrap();
    }

    out
}

pub fn render_github_summary(report: &LintReport) -> String {
    let mut out = render_summary_body(report);
    out.push_str("---\n");
    out.push_str(&format!("*skill-validator v{}*\n", env!("CARGO_PKG_VERSION")));
    out
}

pub fn render_github_comment(report: &LintReport) -> String {
    let mut out = format!("{COMMENT_MARKER}\n");
    out.push_str(&render_summary_body(report));
    out
}
