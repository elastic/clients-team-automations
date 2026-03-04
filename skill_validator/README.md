# skill-validator

A [Trustfall](https://github.com/obi1kenobi/trustfall)-powered linting tool for
validating [Agent Skills](https://agentskills.io/) repositories. Lints are
expressed as declarative Trustfall queries in `.ron` files -- checks are
configuration, not code.

## Installation

### From source

```bash
cargo install --path skill_validator
```

### Release binary

Download a pre-built binary from
[GitHub Releases](https://github.com/elastic/clients-team-automations/releases)
and place it on your `PATH`.

## Usage

```
skill-validator [OPTIONS] [SKILLS_DIR]
```

By default, validates the `./skills` directory using built-in lints.

### Options

| Flag | Description |
|------|-------------|
| `-c, --config <PATH>` | Path to `.skill-validator.toml` (default: `./.skill-validator.toml`) |
| `-f, --format <FMT>` | Output format: `human`, `json`, `github-actions` (default: `human`) |
| `-l, --lint <ID>` | Run only specific lint(s) (repeatable) |
| `--deny <ID>` | Override lint level to Deny (repeatable) |
| `--warn <ID>` | Override lint level to Warn (repeatable) |
| `--allow <ID>` | Override lint level to Allow (repeatable) |
| `--list-lints` | List all available lints and exit |
| `--explain <ID>` | Show detailed explanation for a lint |
| `--scope <SCOPE>` | Validation scope: `all` or `changed` (default: `all`) |
| `--base <REF>` | Base git ref for changed-file detection (default: auto-detect) |
| `--output <PATH>` | Write JSON report to file |
| `--summary <PATH>` | Write Job Summary markdown to file |
| `--comment <PATH>` | Write PR comment markdown to file (includes upsert marker) |
| `-q, --quiet` | Only show errors, suppress warnings |
| `-v, --verbose` | Show detailed diagnostic info |

When the `GITHUB_ACTIONS` environment variable is set, the output format
automatically switches to `github-actions`, producing `::error::` and
`::warning::` annotations for inline PR comments.

### Examples

```bash
# Validate the default skills/ directory
skill-validator

# Validate a specific directory with verbose output
skill-validator -v ./my-skills

# Run only specific lints
skill-validator --lint skill_missing_name --lint skill_flat_layout

# Promote a warning to an error
skill-validator --deny skill_body_too_long
```

### Validating only changed skills

Use `--scope changed` to validate only skills whose files were modified
compared to a base ref. This is useful in CI to avoid re-validating the
entire repository on every PR.

```bash
# Validate only skills changed relative to main
skill-validator --scope changed

# Validate only skills changed relative to a specific branch
skill-validator --scope changed --base origin/release-1.0
```

When `--scope changed` is used without `--base`, the base ref is
auto-detected:

1. If the `GITHUB_BASE_REF` environment variable is set (pull request
   workflows), it uses `origin/$GITHUB_BASE_REF`.
2. Otherwise, it defaults to `main`.

A "changed skill" is any skill directory where at least one file
(SKILL.md, scripts/, references/, etc.) was added, copied, modified, or
renamed in the diff.

## Available lints

Run `skill-validator --list-lints` to see all lints. Run
`skill-validator --explain <ID>` for a detailed explanation of any lint.

### Deny-level (errors)

| Lint ID | Description |
|---------|-------------|
| `skill_flat_layout` | SKILL.md found at wrong nesting depth |
| `skill_missing_frontmatter` | No YAML frontmatter delimiters |
| `skill_missing_name` | Frontmatter missing `name` field |
| `skill_missing_description` | Frontmatter missing `description` field |
| `skill_name_missing_group_prefix` | `name` doesn't start with group folder |
| `skill_name_missing_folder_suffix` | `name` doesn't end with skill folder |
| `skill_name_invalid_format` | `name` is not valid kebab-case |
| `skill_duplicate_name` | Two skills share the same name |
| `skill_mixed_script_languages` | `scripts/` uses multiple languages |

### Warn-level (warnings)

| Lint ID | Description |
|---------|-------------|
| `skill_description_too_short` | Description under 20 words |
| `skill_body_too_long` | Body exceeds 500 lines |
| `skill_missing_examples_section` | No `## Examples` section |
| `skill_missing_guidelines_section` | No `## Guidelines` section |

## Ad-hoc query mode

The same Trustfall adapter that powers lints is exposed as an interactive query
interface. Any question expressible over the
[schema](skills_schema.graphql) can be answered without writing Rust code.

```
skill-validator query [OPTIONS] [SKILLS_DIR]
```

| Flag | Description |
|------|-------------|
| `-q, --query <QUERY>` | Trustfall query string (reads from stdin if omitted) |
| `-a, --args <JSON>` | Query arguments as JSON object (default: `{}`) |
| `-f, --format <FMT>` | Output format: `table`, `json`, `csv` (default: `table`) |
| `--schema` | Print the full GraphQL schema and exit |

### Query examples

**Skills per group folder:**

```bash
skill-validator query -q '{
    GroupFolder {
        name @output
        skill_count @output
    }
}'
```

**Skills with the longest body:**

```bash
skill-validator query -q '{
    Skill {
        skill_file_path @output
        body_line_count @filter(op: ">", value: ["$threshold"]) @output
    }
}' -a '{"threshold": 200}'
```

**Find skills without an Examples section:**

```bash
skill-validator query -q '{
    Skill {
        skill_file_path @output
        section @fold @transform(op: "count") @filter(op: "=", value: ["$zero"]) {
            heading @filter(op: "regex", value: ["$pattern"])
        }
    }
}' -a '{"zero": 0, "pattern": "(?i)^examples?$"}'
```

Discover a pattern interactively, then save the query as a `.ron` lint to
enforce it in CI -- no Rust code changes required.

## GitHub Action

Reference the action from any workflow:

```yaml
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: elastic/clients-team-automations/skill_validator@main
        with:
          skills-dir: skills
```

To validate only skills changed in a PR:

```yaml
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: elastic/clients-team-automations/skill_validator@main
        with:
          scope: changed
```

> **Note:** `fetch-depth: 0` (full history) is required when using
> `scope: changed` so that `git diff` can compare against the base ref.

### PR comments and Job Summary

The action can post a summary comment on the PR and write a
[Job Summary](https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions#adding-a-job-summary)
visible in the Actions tab. The comment is tracked with a hidden marker
(`<!-- skill-validator-bot -->`) and is updated in place on subsequent
pushes rather than creating duplicates.

```yaml
permissions:
  contents: read
  pull-requests: write

jobs:
  validate:
    runs-on: ubuntu-latest
    concurrency:
      group: 'skill-validator-${{ github.event.pull_request.number }}'
      cancel-in-progress: true
    steps:
      - uses: actions/checkout@v4
      - uses: elastic/clients-team-automations/skill_validator@main
        with:
          skills-dir: skills
          post-comment: 'true'
          github-token: ${{ secrets.GITHUB_TOKEN }}
```

> **Concurrency group:** When `post-comment` is enabled, add a
> `concurrency` group keyed on the PR number (as shown above). Without
> it, two runs triggered by rapid pushes can race to create the comment
> and produce duplicates. The concurrency group ensures only the latest
> run completes.

### Inputs

| Input | Default | Description |
|-------|---------|-------------|
| `skills-dir` | `skills` | Path to the skills directory |
| `config` | `.skill-validator.toml` | Path to config file |
| `version` | `latest` | Release version to download (e.g. `v0.1.0`) |
| `scope` | `all` | Validation scope: `all` or `changed` |
| `base` | *(auto-detect)* | Base git ref for changed-file detection (only with `scope: changed`) |
| `extra-args` | | Additional CLI arguments |
| `github-token` | | GitHub token for posting PR comments (requires `pull-requests: write`) |
| `post-comment` | `false` | Post a validation summary comment on the PR |
| `add-summary` | `true` | Write a Job Summary to the Actions tab |

### Outputs

| Output | Description |
|--------|-------------|
| `exit-code` | `0` = pass, `1` = lint failures found |

## Pre-commit hook

Add to your `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/elastic/clients-team-automations
    rev: skill-validator-v0.1.0
    hooks:
      - id: validate-skills
```

Requires the `skill-validator` binary on `PATH` (via `cargo install` or a
release binary).

## Configuration

Create a `.skill-validator.toml` at your repo root:

```toml
# Path to the skills directory (default: "skills")
skills_dir = "skills"

# Per-lint level overrides
[lints]
skill_body_too_long = "deny"
skill_missing_guidelines_section = "allow"

# Directories containing additional .ron lint files
custom_lint_dirs = ["my-lints/"]

# File extensions excluded from mixed-language checks in scripts/
data_extensions = ["txt", "md", "json", "yaml", "yml", "cfg", "ini", "toml", "env", "csv"]
```

### Precedence (highest wins)

1. CLI flags (`--deny`, `--warn`, `--allow`, `--lint`)
2. `.skill-validator.toml` `[lints]` table
3. Built-in defaults from `.ron` files

## Releasing

Releases are driven by git tags. Pushing a tag matching `skill-validator-v*`
triggers the CI workflow which runs tests, cross-compiles binaries for four
targets, and uploads them to a GitHub Release.

| Target | Runner |
|--------|--------|
| `x86_64-unknown-linux-gnu` | ubuntu-latest |
| `aarch64-unknown-linux-gnu` | ubuntu-latest (via `cross`) |
| `x86_64-apple-darwin` | macos-latest |
| `aarch64-apple-darwin` | macos-latest |

### Step-by-step

1. Update the version in `Cargo.toml` and commit:

   ```bash
   # edit Cargo.toml: version = "0.2.0"
   git add Cargo.toml Cargo.lock
   git commit -m "Bump skill-validator to v0.2.0"
   git push
   ```

2. Create and push the tag. The tag **must** use the
   `skill-validator-v<VERSION>` format.

### Using the `gh` CLI

```bash
VERSION=0.2.0

git tag "skill-validator-v${VERSION}"
git push origin "skill-validator-v${VERSION}"

# The release is created automatically by CI once the tag is pushed.
# To create it manually (e.g. with release notes):
gh release create "skill-validator-v${VERSION}" \
  --title "skill-validator v${VERSION}" \
  --generate-notes
```

CI will attach the compiled binaries to the release automatically via
`softprops/action-gh-release`.

### Using the GitHub web UI

1. Go to **Releases** > **Draft a new release**.
2. Click **Choose a tag** and type `skill-validator-v0.2.0` (it will offer
   to create the tag on publish).
3. Set the title to `skill-validator v0.2.0`.
4. Click **Generate release notes** for an automatic changelog, or write
   your own.
5. Click **Publish release**. This creates the tag, which triggers CI to
   build and attach the binaries.
