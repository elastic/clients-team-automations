use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::query::LintLevel;

fn default_skills_dir() -> PathBuf {
    PathBuf::from("skills")
}

fn default_name_pattern() -> String {
    "{{group}}-{{skill}}".to_string()
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

    #[serde(default = "default_name_pattern")]
    pub name_pattern: String,

    #[serde(default)]
    pub lints: BTreeMap<String, LintLevel>,

    #[serde(default)]
    pub custom_lint_dirs: Vec<PathBuf>,

    #[serde(default = "default_data_extensions")]
    pub data_extensions: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            skills_dir: default_skills_dir(),
            name_pattern: default_name_pattern(),
            lints: BTreeMap::new(),
            custom_lint_dirs: Vec::new(),
            data_extensions: default_data_extensions(),
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

    pub fn render_expected_name(&self, group: &str, skill: &str) -> String {
        self.name_pattern
            .replace("{{group}}", group)
            .replace("{{skill}}", skill)
    }

    pub fn is_data_extension(&self, ext: &str) -> bool {
        self.data_extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
    }
}
