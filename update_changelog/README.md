# Update Changelog Action

Generates a changelog from GitHub releases, commits it to a branch, and opens a pull request.

## Usage

```yaml
- name: Checkout repository
  uses: actions/checkout@v4

- name: Update Changelog
  id: changelog
  uses: ./update_changelog
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    title: 'Changelog'
    min_version: '0.2.0'
    changelog_path: 'CHANGELOG.md'
```

## Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `github_token` | GitHub token for API access (requires contents: write, pull-requests: write) | Yes | - |
| `title` | Title for the changelog | No | `Changelog` |
| `min_version` | Minimum semver version to include | No | `0.2.0` |
| `releases_per_page` | Number of releases to fetch | No | `100` |
| `max_version_branch_regex` | Regex for branch-based max version | No | `^\d+\.\d+$` |
| `changelog_path` | Path to the changelog file to update | No | `CHANGELOG.md` |

## Outputs

| Output | Description |
|--------|-------------|
| `changelog` | The generated changelog markdown |
| `pr_url` | URL of the created/updated pull request |
| `pr_number` | Number of the created/updated pull request |
| `branch` | Name of the branch created for the changelog update |

## Features

- Fetches releases from the GitHub API
- Filters by minimum and maximum semver versions
- Sorts releases by date and semver precedence
- Normalizes markdown headings in release bodies
- Generates compare links between versions
- Creates a branch `update-changelog--branches--<base_branch>`
- Commits changes with message `chore(<base_branch>): update changelog`
- Opens a PR (or force-pushes to existing branch/PR)
- Skips update if changelog content is unchanged

## Permissions

The workflow needs the following permissions:

```yaml
permissions:
  contents: write
  pull-requests: write
```
