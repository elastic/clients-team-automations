use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::query::LintLevel;

fn default_skills_dir() -> PathBuf {
    PathBuf::from("skills")
}

fn default_github_org() -> String {
    "elastic".to_string()
}

fn default_data_extensions() -> Vec<String> {
    vec![
        "txt", "md", "json", "yaml", "yml", "cfg", "ini", "toml", "env", "csv",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_skills_dir")]
    pub skills_dir: PathBuf,

    #[serde(default)]
    pub lints: BTreeMap<String, LintLevel>,

    #[serde(default)]
    pub custom_lint_dirs: Vec<PathBuf>,

    #[serde(default = "default_data_extensions")]
    pub data_extensions: Vec<String>,

    #[serde(default = "default_github_org")]
    pub github_org: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            skills_dir: default_skills_dir(),
            lints: BTreeMap::new(),
            custom_lint_dirs: Vec::new(),
            data_extensions: default_data_extensions(),
            github_org: default_github_org(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Self {
        if path.exists() {
            match fs_err::read_to_string(path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        eprintln!("Warning: failed to parse {}: {e}", path.display());
                        Self::default()
                    }
                },
                Err(e) => {
                    eprintln!("Warning: failed to read {}: {e}", path.display());
                    Self::default()
                }
            }
        } else {
            Self::default()
        }
    }

    pub fn is_data_extension(&self, ext: &str) -> bool {
        self.data_extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
    }
}
