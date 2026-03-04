mod adapter;
mod check;
mod config;
mod data;
mod frontmatter;
mod git;
mod markdown;
mod query;
mod query_mode;
mod report;
mod schema;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Scope {
    All,
    Changed,
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scope::All => write!(f, "all"),
            Scope::Changed => write!(f, "changed"),
        }
    }
}

impl std::str::FromStr for Scope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(Scope::All),
            "changed" => Ok(Scope::Changed),
            _ => Err(format!("unknown scope '{s}', expected: all, changed")),
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "skill-validator",
    version,
    about = "Validate Agent Skills repositories",
    args_conflicts_with_subcommands = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[command(flatten)]
    lint_args: LintArgs,
}

#[derive(clap::Args, Debug)]
struct LintArgs {
    /// Path to the skills directory (overrides config, default: ./skills)
    #[arg(value_name = "SKILLS_DIR")]
    skills_dir: Option<PathBuf>,

    /// Path to .skill-validator.toml
    #[arg(short, long, default_value = ".skill-validator.toml")]
    config: PathBuf,

    /// Output format: human, json, github-actions
    #[arg(short, long, default_value = "human")]
    format: report::OutputFormat,

    /// Run only specific lint(s) (repeatable)
    #[arg(short, long = "lint")]
    lints: Vec<String>,

    /// Override lint level to Deny (repeatable)
    #[arg(long)]
    deny: Vec<String>,

    /// Override lint level to Warn (repeatable)
    #[arg(long)]
    warn: Vec<String>,

    /// Override lint level to Allow (repeatable)
    #[arg(long)]
    allow: Vec<String>,

    /// List all available lints and exit
    #[arg(long)]
    list_lints: bool,

    /// Show detailed explanation for a lint
    #[arg(long, value_name = "ID")]
    explain: Option<String>,

    /// Validation scope: all or changed (default: all)
    #[arg(long, default_value = "all")]
    scope: Scope,

    /// Base git ref for changed-file detection (default: auto-detect)
    #[arg(long)]
    base: Option<String>,

    /// Write JSON report to file (used by the GitHub Action for post-processing)
    #[arg(long, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Write GitHub Job Summary markdown to file (append to $GITHUB_STEP_SUMMARY)
    #[arg(long, value_name = "PATH")]
    summary: Option<PathBuf>,

    /// Write PR comment markdown to file (includes marker for upsert)
    #[arg(long, value_name = "PATH")]
    comment: Option<PathBuf>,

    /// Automatically fix auto-fixable issues
    #[arg(long)]
    fix: bool,

    /// Only show errors, suppress warnings
    #[arg(short, long)]
    quiet: bool,

    /// Show detailed diagnostic info
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run an ad-hoc Trustfall query against the skills repository
    Query(QueryArgs),
}

#[derive(clap::Args, Debug)]
struct QueryArgs {
    /// Trustfall query string (reads from stdin if omitted)
    #[arg(short, long)]
    query: Option<String>,

    /// Query arguments as JSON object
    #[arg(short, long, default_value = "{}")]
    args: String,

    /// Output format: table, json, csv
    #[arg(short, long, default_value = "table")]
    format: query_mode::QueryFormat,

    /// Print the full GraphQL schema and exit
    #[arg(long)]
    schema: bool,

    /// Path to the skills directory (overrides config, default: ./skills)
    #[arg(value_name = "SKILLS_DIR")]
    skills_dir: Option<PathBuf>,

    /// Path to .skill-validator.toml
    #[arg(short, long, default_value = ".skill-validator.toml")]
    config: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Query(args)) => run_query_command(args),
        None => run_lint_command(cli.lint_args),
    }
}

fn run_query_command(args: QueryArgs) -> ExitCode {
    if args.schema {
        println!("{}", schema::SCHEMA_TEXT);
        return ExitCode::SUCCESS;
    }

    let cfg = config::Config::load(&args.config);
    let skills_dir = args.skills_dir.unwrap_or_else(|| cfg.skills_dir.clone());

    match query_mode::run_query(
        args.query.as_deref(),
        &args.args,
        &args.format,
        &skills_dir,
        &cfg,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::from(2)
        }
    }
}

fn run_lint_command(args: LintArgs) -> ExitCode {
    let auto_format = std::env::var("GITHUB_ACTIONS").is_ok();
    let format = if auto_format && args.format == report::OutputFormat::Human {
        report::OutputFormat::GithubActions
    } else {
        args.format.clone()
    };

    let all_lints = query::load_builtin_lints();

    if args.list_lints {
        for lint in &all_lints {
            println!(
                "{:<45} [{}] {}",
                lint.id,
                match lint.lint_level {
                    query::LintLevel::Deny => "deny",
                    query::LintLevel::Warn => "warn",
                    query::LintLevel::Allow => "allow",
                },
                lint.human_readable_name,
            );
        }
        return ExitCode::SUCCESS;
    }

    if let Some(id) = &args.explain {
        match all_lints.iter().find(|l| &l.id == id) {
            Some(lint) => {
                println!("{}", lint.human_readable_name);
                println!("ID: {}", lint.id);
                println!("Level: {:?}", lint.lint_level);
                println!();
                println!("{}", lint.description);
                if let Some(ref link) = lint.reference_link {
                    println!();
                    println!("Reference: {link}");
                }
            }
            None => {
                eprintln!("Unknown lint: {id}");
                return ExitCode::from(2);
            }
        }
        return ExitCode::SUCCESS;
    }

    let cfg = config::Config::load(&args.config);
    let skills_dir = args.skills_dir.unwrap_or_else(|| cfg.skills_dir.clone());

    let scope_filter = if args.scope == Scope::Changed {
        let repo_root = match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Error: cannot get current directory: {e}");
                return ExitCode::from(2);
            }
        };
        let base_ref = git::resolve_base_ref(args.base.as_deref());
        if args.verbose {
            eprintln!("Scope: changed (base ref: {base_ref})");
        }
        match git::changed_skill_dirs(&base_ref, &skills_dir, &repo_root) {
            Ok(dirs) => {
                if args.verbose {
                    eprintln!("Changed skill directories: {}", dirs.len());
                    for d in &dirs {
                        eprintln!("  {d}");
                    }
                }
                if dirs.is_empty() {
                    eprintln!("No changed skills detected — nothing to validate.");
                    return ExitCode::SUCCESS;
                }
                Some(dirs)
            }
            Err(e) => {
                eprintln!("Error: {e}");
                return ExitCode::from(2);
            }
        }
    } else {
        None
    };

    let overrides = query::LintLevelOverrides {
        deny: args.deny.into_iter().collect(),
        warn: args.warn.into_iter().collect(),
        allow: args.allow.into_iter().collect(),
    };

    match check::run_all_lints(
        &skills_dir,
        &cfg,
        &all_lints,
        &overrides,
        &args.lints,
        args.quiet,
        scope_filter.as_ref(),
    ) {
        Ok(lint_report) => {
            if args.fix {
                eprintln!("No auto-fixable issues available. All lints are report-only.");
            }
            report::print_report(&lint_report, &format, args.verbose);

            if let Some(ref path) = args.output {
                if let Err(e) = report::write_json_file(&lint_report, path) {
                    eprintln!("Warning: failed to write JSON report to {}: {e}", path.display());
                }
            }

            if let Some(ref path) = args.summary {
                let md = report::render_github_summary(&lint_report);
                if let Err(e) = fs_err::write(path, &md) {
                    eprintln!("Warning: failed to write summary to {}: {e}", path.display());
                }
            }

            if let Some(ref path) = args.comment {
                let md = report::render_github_comment(&lint_report);
                if let Err(e) = fs_err::write(path, &md) {
                    eprintln!("Warning: failed to write comment to {}: {e}", path.display());
                }
            }

            if lint_report.has_errors() {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::from(2)
        }
    }
}
