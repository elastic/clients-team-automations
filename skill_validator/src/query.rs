use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use trustfall::TransparentValue;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LintLevel {
    #[serde(alias = "deny")]
    Deny,
    #[serde(alias = "warn")]
    Warn,
    #[serde(alias = "allow")]
    Allow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLint {
    pub id: String,
    pub human_readable_name: String,
    pub description: String,
    pub lint_level: LintLevel,

    #[serde(default)]
    pub reference_link: Option<String>,

    pub query: String,

    #[serde(default)]
    pub arguments: BTreeMap<String, TransparentValue>,

    pub error_message: String,

    #[serde(default)]
    pub per_result_error_template: Option<String>,
}

#[derive(Debug, Default)]
pub struct LintLevelOverrides {
    pub deny: HashSet<String>,
    pub warn: HashSet<String>,
    pub allow: HashSet<String>,
}

impl SkillLint {
    pub fn effective_level(
        &self,
        config_overrides: &BTreeMap<String, LintLevel>,
        cli_overrides: &LintLevelOverrides,
    ) -> LintLevel {
        if cli_overrides.deny.contains(&self.id) {
            return LintLevel::Deny;
        }
        if cli_overrides.warn.contains(&self.id) {
            return LintLevel::Warn;
        }
        if cli_overrides.allow.contains(&self.id) {
            return LintLevel::Allow;
        }
        if let Some(level) = config_overrides.get(&self.id) {
            return level.clone();
        }
        self.lint_level.clone()
    }
}

/// Returns the path to the built-in lint `.ron` files shipped in this repo.
/// Only meaningful on the build machine; intended for use in tests.
pub fn builtin_lint_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lints")
}

pub fn load_lints(dirs: &[std::path::PathBuf]) -> Vec<SkillLint> {
    let mut lints = Vec::new();
    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("ron") {
                    match fs_err::read_to_string(&path) {
                        Ok(content) => match ron::from_str::<SkillLint>(&content) {
                            Ok(lint) => lints.push(lint),
                            Err(e) => {
                                eprintln!(
                                    "Warning: failed to parse lint {}: {e}",
                                    path.display()
                                );
                            }
                        },
                        Err(e) => {
                            eprintln!(
                                "Warning: failed to read {}: {e}",
                                path.display()
                            );
                        }
                    }
                }
            }
        }
    }
    lints
}
