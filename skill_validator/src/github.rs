use std::sync::Arc;

use crate::data::{GitHubOrgData, GitHubTeamData};

fn resolve_token() -> Option<String> {
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        if !t.is_empty() {
            return Some(t);
        }
    }
    if let Ok(t) = std::env::var("GH_TOKEN") {
        if !t.is_empty() {
            return Some(t);
        }
    }

    // Fall back to `gh auth token` for local dev
    std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|t| !t.is_empty())
}

#[derive(serde::Deserialize)]
struct ApiTeam {
    slug: String,
    name: String,
    description: Option<String>,
}

fn parse_next_url(link_header: &str) -> Option<String> {
    for part in link_header.split(',') {
        let part = part.trim();
        if part.ends_with("rel=\"next\"") {
            if let (Some(start), Some(end)) = (part.find('<'), part.find('>')) {
                return Some(part[start + 1..end].to_string());
            }
        }
    }
    None
}

fn fetch_teams_paginated(token: &str, org: &str) -> Result<Vec<Arc<GitHubTeamData>>, String> {
    let agent = ureq::Agent::new();
    let mut teams = Vec::new();
    let mut url = format!("https://api.github.com/orgs/{org}/teams?per_page=100");

    loop {
        let resp = agent
            .get(&url)
            .set("Authorization", &format!("Bearer {token}"))
            .set("Accept", "application/vnd.github+json")
            .set("User-Agent", "skill-validator")
            .set("X-GitHub-Api-Version", "2022-11-28")
            .call()
            .map_err(|e| format!("GitHub API request failed: {e}"))?;

        let next_url = resp
            .header("Link")
            .and_then(parse_next_url);

        let body: Vec<ApiTeam> = resp
            .into_json()
            .map_err(|e| format!("Failed to parse GitHub API response: {e}"))?;

        teams.extend(body.into_iter().map(|t| {
            Arc::new(GitHubTeamData {
                slug: t.slug,
                name: t.name,
                description: t.description,
            })
        }));

        match next_url {
            Some(u) => url = u,
            None => break,
        }
    }

    Ok(teams)
}

pub fn fetch_org_data(org: &str) -> GitHubOrgData {
    let token = match resolve_token() {
        Some(t) => t,
        None => {
            eprintln!(
                "Note: No GitHub token found (checked GITHUB_TOKEN, GH_TOKEN, `gh auth token`). \
                 GitHub-dependent lints will be skipped."
            );
            return GitHubOrgData {
                name: org.to_string(),
                teams_loaded: false,
                teams: Vec::new(),
            };
        }
    };

    match fetch_teams_paginated(&token, org) {
        Ok(teams) => {
            let count = teams.len();
            eprintln!("Loaded {count} GitHub teams from {org}");
            GitHubOrgData {
                name: org.to_string(),
                teams_loaded: true,
                teams,
            }
        }
        Err(e) => {
            eprintln!("Warning: failed to fetch GitHub teams for {org}: {e}");
            GitHubOrgData {
                name: org.to_string(),
                teams_loaded: false,
                teams: Vec::new(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_next_url_extracts_next_link() {
        let header = r#"<https://api.github.com/orgs/elastic/teams?page=2&per_page=100>; rel="next", <https://api.github.com/orgs/elastic/teams?page=5&per_page=100>; rel="last""#;
        assert_eq!(
            parse_next_url(header),
            Some("https://api.github.com/orgs/elastic/teams?page=2&per_page=100".to_string())
        );
    }

    #[test]
    fn parse_next_url_returns_none_when_no_next() {
        let header = r#"<https://api.github.com/orgs/elastic/teams?page=1&per_page=100>; rel="first", <https://api.github.com/orgs/elastic/teams?page=5&per_page=100>; rel="last""#;
        assert_eq!(parse_next_url(header), None);
    }

    #[test]
    fn parse_next_url_returns_none_for_empty() {
        assert_eq!(parse_next_url(""), None);
    }

    #[test]
    fn no_token_returns_unloaded_org() {
        // Clear env vars to ensure no token is found via env
        let orig_gh = std::env::var("GITHUB_TOKEN").ok();
        let orig_gh2 = std::env::var("GH_TOKEN").ok();
        std::env::remove_var("GITHUB_TOKEN");
        std::env::remove_var("GH_TOKEN");

        // We can't easily mock `gh auth token`, so just verify the struct shape
        // when called without any env token (may still succeed via gh CLI)
        let data = fetch_org_data("nonexistent-org-for-test");
        assert_eq!(data.name, "nonexistent-org-for-test");
        // Restore
        if let Some(v) = orig_gh {
            std::env::set_var("GITHUB_TOKEN", v);
        }
        if let Some(v) = orig_gh2 {
            std::env::set_var("GH_TOKEN", v);
        }
    }
}
