mod adapter;
mod check;
mod config;
mod data;
mod frontmatter;
mod markdown;
mod query;
mod report;
mod schema;

use clap::Parser;
use std::path::PathBuf;
use std::process;

#[derive(Parser, Debug)]
#[command(name = "skill-validator", version, about = "Validate Agent Skills repositories")]
struct Cli {
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

    /// Only show errors, suppress warnings
    #[arg(short, long)]
    quiet: bool,

    /// Show detailed diagnostic info
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    let auto_format = std::env::var("GITHUB_ACTIONS").is_ok();
    let format = if auto_format && cli.format == report::OutputFormat::Human {
        report::OutputFormat::GithubActions
    } else {
        cli.format.clone()
    };

    let all_lints = query::load_builtin_lints();

    if cli.list_lints {
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
        return;
    }

    if let Some(id) = &cli.explain {
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
                process::exit(2);
            }
        }
        return;
    }

    let cfg = config::Config::load(&cli.config);

    let skills_dir = cli.skills_dir.unwrap_or_else(|| cfg.skills_dir.clone());

    let overrides = query::LintLevelOverrides {
        deny: cli.deny.into_iter().collect(),
        warn: cli.warn.into_iter().collect(),
        allow: cli.allow.into_iter().collect(),
    };

    match check::run_all_lints(
        &skills_dir,
        &cfg,
        &all_lints,
        &overrides,
        &cli.lints,
        cli.quiet,
    ) {
        Ok(lint_report) => {
            report::print_report(&lint_report, &format, cli.verbose);
            if lint_report.has_errors() {
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(2);
        }
    }
}
