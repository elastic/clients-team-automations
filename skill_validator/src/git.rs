use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use anyhow::Context;

/// Resolve the base ref for changed-file detection.
///
/// Precedence: explicit `--base` flag > `GITHUB_BASE_REF` env var > `"origin/main"`.
pub fn resolve_base_ref(explicit: Option<&str>) -> String {
    if let Some(base) = explicit {
        return base.to_string();
    }
    if let Ok(gh_base) = std::env::var("GITHUB_BASE_REF") {
        if !gh_base.is_empty() {
            return format!("origin/{gh_base}");
        }
    }
    "origin/main".to_string()
}

/// Derive the set of skill directory paths from an explicit list of file paths.
///
/// Used in pre-commit mode where the hook runner passes the staged file paths
/// directly. Each path is mapped to its enclosing skill directory at depth 2
/// under `skills_prefix` (i.e. `<group>/<skill-name>/`). Paths outside
/// `skills_prefix` or at insufficient depth are ignored.
pub fn skill_dirs_from_files(files: &[std::path::PathBuf], skills_prefix: &str) -> HashSet<String> {
    let mut dirs = HashSet::new();
    for file in files {
        let file_str = file.to_string_lossy();
        let line = file_str.trim();
        if let Some(rel) = strip_prefix_normalized(line, skills_prefix) {
            let components: Vec<&str> = rel.split('/').collect();
            if components.len() >= 2 {
                let skill_dir = format!("{}/{}/{}", skills_prefix, components[0], components[1]);
                dirs.insert(skill_dir);
            }
        }
    }
    dirs
}

/// Return the set of skill directory paths (relative to `repo_root`) that have
/// changed files compared to `base_ref`.
///
/// A changed file is mapped to its enclosing skill directory at depth 2 under
/// `skills_dir` (i.e. `<group>/<skill-name>/`). Files outside `skills_dir` or
/// at insufficient depth are ignored.
pub fn changed_skill_dirs(
    base_ref: &str,
    skills_dir: &Path,
    repo_root: &Path,
) -> anyhow::Result<HashSet<String>> {
    let output = Command::new("git")
        .args([
            "diff",
            "--name-only",
            "--diff-filter=ACMR",
            &format!("{base_ref}...HEAD"),
        ])
        .current_dir(repo_root)
        .output()
        .context("failed to run git diff")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git diff failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let raw_prefix = skills_dir.to_string_lossy();
    let skills_prefix = raw_prefix.trim_end_matches('/');

    let mut dirs = HashSet::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Only consider files under skills_dir
        let rel = match strip_prefix_normalized(line, skills_prefix) {
            Some(r) => r,
            None => continue,
        };

        // Map to skill dir at depth 2: <group>/<skill-name>/...
        let components: Vec<&str> = rel.split('/').collect();
        if components.len() >= 2 {
            let skill_dir = format!("{}/{}/{}", skills_prefix, components[0], components[1]);
            dirs.insert(skill_dir);
        }
    }

    Ok(dirs)
}

fn strip_prefix_normalized<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    let path = path.trim_start_matches("./");
    let prefix = prefix.trim_start_matches("./");

    path.strip_prefix(prefix)
        .map(|rest| rest.trim_start_matches('/'))
        .filter(|rest| !rest.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_prefix_basic() {
        assert_eq!(
            strip_prefix_normalized("skills/es/query/SKILL.md", "skills"),
            Some("es/query/SKILL.md")
        );
    }

    #[test]
    fn strip_prefix_with_dot_slash() {
        assert_eq!(
            strip_prefix_normalized("./skills/es/query/SKILL.md", "skills"),
            Some("es/query/SKILL.md")
        );
        assert_eq!(
            strip_prefix_normalized("skills/es/query/SKILL.md", "./skills"),
            Some("es/query/SKILL.md")
        );
    }

    #[test]
    fn strip_prefix_no_match() {
        assert_eq!(
            strip_prefix_normalized("other/file.txt", "skills"),
            None
        );
    }

    #[test]
    fn strip_prefix_exact_match_returns_none() {
        assert_eq!(strip_prefix_normalized("skills", "skills"), None);
    }

    #[test]
    fn skill_dirs_from_files_basic() {
        let files: Vec<std::path::PathBuf> = vec![
            "skills/es/query/SKILL.md".into(),
            "skills/es/query/scripts/helper.js".into(),
            "skills/other/group/skill/file.txt".into(), // depth 3+ → maps to skills/other/group
            "unrelated/file.txt".into(),
        ];
        let dirs = skill_dirs_from_files(&files, "skills");
        assert!(dirs.contains("skills/es/query"), "should include es/query");
        assert!(dirs.contains("skills/other/group"), "should include other/group");
        assert!(!dirs.contains("skills/other/group/skill"), "should not go deeper than depth 2");
        assert!(!dirs.iter().any(|d| d.contains("unrelated")), "unrelated files should be excluded");
    }

    #[test]
    fn skill_dirs_from_files_empty() {
        let files: Vec<std::path::PathBuf> = vec![];
        let dirs = skill_dirs_from_files(&files, "skills");
        assert!(dirs.is_empty());
    }

    #[test]
    fn skill_dirs_from_files_no_matches() {
        let files: Vec<std::path::PathBuf> = vec!["README.md".into(), "other/file.txt".into()];
        let dirs = skill_dirs_from_files(&files, "skills");
        assert!(dirs.is_empty());
    }
}
