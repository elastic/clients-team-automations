use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use trustfall::TransparentValue;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LintLevel {
    Deny,
    Warn,
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

macro_rules! include_lint {
    ($file:expr) => {{
        let ron_str = include_str!(concat!("lints/", $file));
        ron::from_str::<SkillLint>(ron_str)
            .unwrap_or_else(|e| panic!("failed to parse lint {}: {e}", $file))
    }};
}

pub fn load_builtin_lints() -> Vec<SkillLint> {
    vec![
        // Deny-level
        include_lint!("skill_flat_layout.ron"),
        include_lint!("skill_missing_frontmatter.ron"),
        include_lint!("skill_missing_name.ron"),
        include_lint!("skill_missing_description.ron"),
        include_lint!("skill_name_mismatch.ron"),
        include_lint!("skill_name_invalid_format.ron"),
        include_lint!("skill_duplicate_name.ron"),
        include_lint!("skill_mixed_script_languages.ron"),
        include_lint!("skill_name_consecutive_hyphens.ron"),
        include_lint!("skill_name_too_long.ron"),
        include_lint!("skill_description_too_long.ron"),
        // Warn-level
        include_lint!("skill_description_too_short.ron"),
        include_lint!("skill_body_too_long.ron"),
        include_lint!("skill_missing_examples_section.ron"),
        include_lint!("skill_missing_guidelines_section.ron"),
        // Allow-level (opt-in)
        include_lint!("skill_has_no_scripts.ron"),
        include_lint!("skill_has_no_references.ron"),
    ]
}

pub fn load_custom_lints(dirs: &[std::path::PathBuf]) -> Vec<SkillLint> {
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
