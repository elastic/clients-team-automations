use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

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
) -> Result<HashSet<String>, String> {
    let output = Command::new("git")
        .args([
            "diff",
            "--name-only",
            "--diff-filter=ACMR",
            &format!("{base_ref}...HEAD"),
        ])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("failed to run git diff: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git diff failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let skills_prefix = skills_dir.to_string_lossy();

    let mut dirs = HashSet::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Only consider files under skills_dir
        let rel = match strip_prefix_normalized(line, &skills_prefix) {
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
}
