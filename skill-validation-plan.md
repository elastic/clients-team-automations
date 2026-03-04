# Skill Validator — Implementation Plan

A Trustfall-powered linting tool for validating [Agent Skills](https://agentskills.io/)
repositories, inspired by the architecture of `cargo-semver-checks`. Lints are expressed as
declarative Trustfall queries in `.ron` files — checks are configuration, not code.

## 1. Goals

| Goal | Detail |
|------|--------|
| **Replace `validate-skills.sh`** | Port every check in `agent-skills-sandbox/scripts/validate-skills.sh` to a declarative lint |
| **Extensible** | Adding a new lint = adding a new `.ron` file, zero code changes |
| **Triple distribution** | CLI binary, GitHub Action, and pre-commit hook |
| **Spec-aware** | Validate against the [agentskills.io specification](https://agentskills.io/specification) |
| **Great diagnostics** | Per-finding file paths, line numbers, and human-readable explanations |

## 2. Repository layout within `clients-team-automations`

This repo is a **mono-repo of reusable GitHub Actions**. Each action lives in its own
top-level directory (`gencheck/`, `update_changelog/`, `issues_stats/`). The skill validator
follows the same convention — it gets its own `skill_validator/` directory.

> **Existing repo pattern (for reference):**
>
> | Directory | Language | `action.yml` location | Implementation |
> |-----------|----------|----------------------|----------------|
> | `gencheck/` | TypeScript | `gencheck/action.yml` | `gencheck/check/` |
> | `update_changelog/` | TypeScript | `update_changelog/action.yml` | `update_changelog/action/` |
> | `issues_stats/` | JavaScript | `issues_stats/action.yml` | `issues_stats/ingest/` |
> | **`skill_validator/`** | **Rust** | **`skill_validator/action.yml`** | **`skill_validator/` (Cargo project root)** |

```text
clients-team-automations/                 # repo root (shared with other actions)
├── .github/
│   └── workflows/
│       ├── gencheck.yml                  # existing — tests gencheck action
│       ├── update-changelog.yml          # existing — tests update_changelog action
│       ├── issues_stats_tests.yml        # existing — tests issues_stats action
│       └── skill-validator.yml           # NEW — CI for skill_validator (cargo test, etc.)
├── .pre-commit-hooks.yaml                # NEW — pre-commit hook definition (must be at repo root)
├── gencheck/                             # existing action
├── update_changelog/                     # existing action
├── issues_stats/                         # existing action
├── testcheck/                            # existing action
└── skill_validator/                      # NEW — skill validator action + Rust crate
    ├── action.yml                        # GitHub Action metadata (composite)
    ├── Cargo.toml
    ├── README.md
    ├── .gitignore                        # Rust-specific ignores (target/, etc.)
    ├── skills_schema.graphql             # Trustfall schema (shipped for editor LSP + query mode)
    ├── src/
    │   ├── main.rs                       # CLI entry point (clap): lint + query subcommands
    │   ├── lib.rs                        # Public API
    │   ├── query.rs                      # Lint loading, SkillLint struct, macro
    │   ├── query_mode.rs                 # Ad-hoc query execution and output formatting
    │   ├── check.rs                      # Orchestration: load data → run lints → report
    │   ├── adapter/
    │   │   ├── mod.rs                    # SkillsAdapter (implements Trustfall BasicAdapter)
    │   │   └── vertex.rs                 # Vertex enum and helpers
    │   ├── schema.rs                     # Embeds skills_schema.graphql via include_str!
    │   ├── data.rs                       # Skill data model (parsed from filesystem)
    │   ├── config.rs                     # .skill-validator.toml loading and defaults
    │   ├── frontmatter.rs                # YAML frontmatter parser
    │   ├── markdown.rs                   # Markdown structure parser (headings, sections, code blocks)
    │   ├── report.rs                     # Human-readable + GitHub Actions error output
    │   └── lints/
    │       ├── skill_missing_name.ron
    │       ├── skill_missing_description.ron
    │       ├── skill_name_missing_group_prefix.ron
    │       ├── skill_name_missing_folder_suffix.ron
    │       ├── skill_name_invalid_format.ron
    │       ├── skill_duplicate_name.ron
    │       ├── skill_flat_layout.ron
    │       ├── skill_missing_frontmatter.ron
    │       ├── skill_description_too_short.ron
    │       ├── skill_body_too_long.ron
    │       ├── skill_missing_examples_section.ron
    │       ├── skill_missing_guidelines_section.ron
    │       ├── skill_mixed_script_languages.ron
    │       └── ...
    └── test_crates/                      # Test skill repos for snapshot testing
        ├── valid_skill/
        ├── missing_name/
        ├── flat_layout/
        └── ...
```

### Key differences from sibling actions

| Concern | Existing actions (TypeScript/JS) | skill_validator (Rust) |
|---------|----------------------------------|------------------------|
| Runtime | Node.js (installed in composite step) | Pre-built binary (downloaded in composite step) |
| Build in CI | `npm ci && npm run build && node dist/index.js` | `cargo test` in CI; binary pre-built for releases |
| Artifacts checked in | `dist/` (compiled JS) | Nothing — binary distributed via GitHub Releases |
| `.gitignore` | Root-level ignores `node_modules`, `dist`, `*.d.ts` | `skill_validator/.gitignore` ignores `target/` |

## 3. Trustfall Schema

This is the core design decision. The schema models skills as a queryable graph.

```graphql
schema {
    query: RootQuery
}

"""
Entry point for querying skills in a repository.
"""
type RootQuery {
    """
    All valid skills in the repository (SKILL.md at correct nesting depth).
    """
    Skill: [Skill!]!

    """
    All group folders directly under the skills root.
    """
    GroupFolder: [GroupFolder!]!

    """
    Every SKILL.md file discovered in the repository, regardless of location.
    Includes files at valid depths and invalid depths (flat layout, too deep, etc.).
    Lints filter on `depth` to detect structural violations.
    """
    DiscoveredSkillFile: [DiscoveredSkillFile!]!
}

# ---------------------------------------------------------------------------
# Core types
# ---------------------------------------------------------------------------

"""
A skill discovered in the repository.
A valid skill is a directory containing a SKILL.md file at the correct nesting depth.
"""
type Skill {
    # ---- Identity & location ----

    """The directory name of the skill folder (e.g. "query-authoring")."""
    folder_name: String!

    """The group folder name this skill is nested under (e.g. "elasticsearch")."""
    group_folder: String!

    """Relative path from the repo root to the skill directory."""
    path: String!

    """Relative path from the repo root to the SKILL.md file."""
    skill_file_path: String!

    """Nesting depth under the skills root (valid skills are depth >= 2)."""
    depth: Int!

    # ---- Frontmatter properties ----

    """Whether the SKILL.md file has valid YAML frontmatter delimiters."""
    has_frontmatter: Boolean!

    """The raw YAML frontmatter text, if present."""
    raw_frontmatter: String

    """The `name` field from frontmatter, if present."""
    name: String

    """The `description` field from frontmatter, if present."""
    description: String

    """The `license` field from frontmatter, if present."""
    license: String

    """The `compatibility` field from frontmatter, if present."""
    compatibility: String

    """The `allowed-tools` field from frontmatter, if present."""
    allowed_tools: String

    """Character count of the description field."""
    description_length: Int!

    """Word count of the description field."""
    description_word_count: Int!

    # ---- Body properties ----

    """Total line count of the SKILL.md file."""
    total_line_count: Int!

    """Line count of the markdown body (excluding frontmatter)."""
    body_line_count: Int!

    """Whether the body starts with a level-1 heading."""
    has_title_heading: Boolean!

    """The text of the first level-1 heading, if present."""
    title_heading: String

    # ---- Edges ----

    """Frontmatter metadata key-value pairs."""
    metadata: [MetadataEntry!]!

    """All markdown sections (## headings) in the body."""
    section: [Section!]!

    """All subdirectories of this skill (scripts/, references/, assets/, etc.)."""
    sub_dir: [SubDir!]!

    """
    All other skills in the repository (excluding this one).
    Enables cross-skill comparisons (duplicate names, overlapping descriptions, etc.)
    entirely within .ron queries using @tag + @filter.
    """
    all_other_skills: [Skill!]!

    """Span information for the SKILL.md file (for error reporting)."""
    span: Span!

    """Span of the frontmatter block within SKILL.md."""
    frontmatter_span: Span
}

"""
A grouping folder directly under the skills root (e.g. "elasticsearch", "kibana").
"""
type GroupFolder {
    """The folder name."""
    name: String!

    """Relative path from the repo root."""
    path: String!

    """Skills nested under this group folder."""
    skill: [Skill!]!

    """Count of skills in this group."""
    skill_count: Int!
}

"""
A SKILL.md file discovered anywhere in the repository tree.
Not all discovered files are valid skills — some may be at wrong depths.
"""
type DiscoveredSkillFile {
    """Relative path from the repo root."""
    path: String!

    """The parent directory name."""
    parent_dir: String!

    """Nesting depth under the skills root."""
    depth: Int!

    """The resolved Skill, if this file is at a valid location and parseable."""
    skill: Skill

    span: Span!
}

# ---------------------------------------------------------------------------
# Markdown structure
# ---------------------------------------------------------------------------

"""
A markdown section identified by a heading (## level).
"""
type Section {
    """The heading level (1 = #, 2 = ##, etc.)."""
    level: Int!

    """The heading text (without the # prefix)."""
    heading: String!

    """The line number where this heading appears."""
    line_number: Int!

    """The full text content under this heading (until the next heading of same or higher level)."""
    content: String!

    """Line count of this section's content."""
    content_line_count: Int!

    """Code blocks within this section."""
    code_block: [CodeBlock!]!
}

"""
A fenced code block within markdown.
"""
type CodeBlock {
    """The language tag (e.g. "json", "yaml", "bash"), if specified."""
    language: String

    """Whether a language tag is present."""
    has_language_tag: Boolean!

    """The line number where the code block starts."""
    line_number: Int!

    """The content of the code block."""
    content: String!
}

# ---------------------------------------------------------------------------
# Filesystem structure (generic)
# ---------------------------------------------------------------------------

"""
A subdirectory of a skill (scripts/, references/, assets/, or any future subdirectory).
Replaces the former ScriptsDir/ReferencesDir/AssetsDir — one generic type for all.
"""
type SubDir {
    """The directory name (e.g. "scripts", "references", "assets")."""
    name: String!

    """Relative path from the repo root."""
    path: String!

    """All files in this subdirectory."""
    file: [SubDirFile!]!

    """Total number of files."""
    file_count: Int!

    """Unique file extensions across all files."""
    unique_extensions: [String!]!
}

"""
A file inside a skill subdirectory.
Replaces the former ScriptFile/ReferenceFile/AssetFile — one generic type for all.
"""
type SubDirFile {
    """File name."""
    name: String!

    """File extension."""
    extension: String!

    """Relative path from the repo root."""
    path: String!

    """
    Whether this file is classified as a data file (e.g. .json, .yaml, .csv).
    Classification is driven by the `data_extensions` list in .skill-validator.toml.
    Useful for filtering when counting script languages (exclude data files).
    """
    is_data_file: Boolean!
}

# ---------------------------------------------------------------------------
# Metadata & utility types
# ---------------------------------------------------------------------------

"""
A key-value pair from the frontmatter `metadata` map.
"""
type MetadataEntry {
    key: String!
    value: String!
}

"""
Source location information for error reporting.
"""
type Span {
    """Relative file path from the repo root."""
    filename: String!

    """1-based starting line number."""
    begin_line: Int!

    """1-based ending line number."""
    end_line: Int!
}
```

### Schema design rationale

| Decision | Rationale |
|----------|-----------|
| **`Skill` as the primary type** | Every lint is fundamentally about a skill — this keeps queries simple |
| **Frontmatter fields as properties** | Enables `@filter` and `@output` directly, no extra traversal |
| **Naming convention expressed in `.ron` queries** | Instead of a pre-computed `expected_name`, lints use `@tag` + `has_prefix`/`has_suffix` on `group_folder` and `folder_name`. The convention is fully declarative — repos that don't use group prefixes simply disable the prefix lint |
| **`Section` edge instead of flat markdown** | Lets lints query for specific sections (e.g., "does an Examples section exist?") |
| **`DiscoveredSkillFile` instead of `InvalidSkillFile`** | Emits ALL found SKILL.md files; lints filter on `depth`. The "what is invalid" policy lives in the `.ron` query, not baked into the adapter |
| **`all_other_skills` edge instead of `DuplicateSkillName`** | Cross-skill comparisons (duplicate names, overlapping descriptions, etc.) are expressed via `@tag` + `@filter` in `.ron` queries. No new Rust type needed per comparison — the adapter just provides the generic edge. O(n²) is acceptable for skill repos (< 500 skills) |
| **`SubDir` + `SubDirFile` instead of 6 directory types** | `ScriptsDir`, `ReferencesDir`, `AssetsDir` and their file types were structurally identical. One generic pair covers all subdirectories. `is_data_file` (config-driven) replaces `is_script`. Future subdirectories (e.g. `templates/`) need zero schema changes |
| **`Span` type throughout** | Consistent error reporting with file + line info, matching cargo-semver-checks conventions |
| **No diff/comparison** | Unlike cargo-semver-checks, we validate a single snapshot, not a before/after. The schema reflects this: no `CrateDiff` equivalent needed |

## 4. Example Lints (.ron files)

### 4.1 `skill_missing_frontmatter.ron`

```ron
SkillLint(
    id: "skill_missing_frontmatter",
    human_readable_name: "SKILL.md missing YAML frontmatter",
    description: "A SKILL.md file does not have valid YAML frontmatter delimited by --- markers.",
    lint_level: Deny,
    reference_link: Some("https://agentskills.io/specification#frontmatter-required"),
    query: r#"
    {
        Skill {
            has_frontmatter @filter(op: "=", value: ["$false"])

            skill_file_path @output
            group_folder @output
            folder_name @output

            span_: span {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {
        "false": false,
    },
    error_message: "A SKILL.md file is missing YAML frontmatter. Every SKILL.md must begin with --- delimited YAML frontmatter containing at least `name` and `description` fields.",
    per_result_error_template: Some("{{skill_file_path}} has no YAML frontmatter"),
)
```

### 4.2 `skill_missing_name.ron`

```ron
SkillLint(
    id: "skill_missing_name",
    human_readable_name: "SKILL.md frontmatter missing name",
    description: "A SKILL.md file's frontmatter is missing the required `name` field.",
    lint_level: Deny,
    reference_link: Some("https://agentskills.io/specification#name-field"),
    query: r#"
    {
        Skill {
            has_frontmatter @filter(op: "=", value: ["$true"])
            name @filter(op: "is_null")

            skill_file_path @output
            group_folder @output
            folder_name @output

            span_: frontmatter_span @optional {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {
        "true": true,
    },
    error_message: "A SKILL.md file's frontmatter is missing the required `name` field.",
    per_result_error_template: Some("{{skill_file_path}} frontmatter is missing required field: name"),
)
```

### 4.3 `skill_name_missing_group_prefix.ron` and `skill_name_missing_folder_suffix.ron`

Instead of a single `skill_name_mismatch` lint that relied on a pre-computed `expected_name`,
the naming convention is expressed as two focused lints using `@tag` + `has_prefix`/`has_suffix`
on the existing `group_folder` and `folder_name` properties. Repos that don't use group
prefixes simply disable the prefix lint.

```ron
SkillLint(
    id: "skill_name_missing_group_prefix",
    human_readable_name: "skill name missing group folder prefix",
    description: "The frontmatter `name` should start with the group folder name.",
    lint_level: Deny,
    reference_link: Some("https://agentskills.io/specification#name-field"),
    query: r#"
    {
        Skill {
            has_frontmatter @filter(op: "=", value: ["$true"])
            name @filter(op: "is_not_null")
                 @filter(op: "not_has_prefix", value: ["%group"])
                 @output

            group_folder @tag(name: "group") @output
            skill_file_path @output

            span_: frontmatter_span @optional {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {
        "true": true,
    },
    error_message: "The `name` field does not start with the group folder name.",
    per_result_error_template: Some("{{skill_file_path}} name '{{name}}' does not start with group '{{group_folder}}'"),
)
```

```ron
SkillLint(
    id: "skill_name_missing_folder_suffix",
    human_readable_name: "skill name missing skill folder suffix",
    description: "The frontmatter `name` should end with the skill folder name.",
    lint_level: Deny,
    reference_link: Some("https://agentskills.io/specification#name-field"),
    query: r#"
    {
        Skill {
            has_frontmatter @filter(op: "=", value: ["$true"])
            name @filter(op: "is_not_null")
                 @filter(op: "not_has_suffix", value: ["%folder"])
                 @output

            folder_name @tag(name: "folder") @output
            skill_file_path @output

            span_: frontmatter_span @optional {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {
        "true": true,
    },
    error_message: "The `name` field does not end with the skill folder name.",
    per_result_error_template: Some("{{skill_file_path}} name '{{name}}' does not end with folder '{{folder_name}}'"),
)
```

### 4.4 `skill_flat_layout.ron`

Uses the generic `DiscoveredSkillFile` entry point — the depth threshold lives in the
query arguments, not in the adapter. The query filters on `depth < $min_depth`, so
`skills/<folder>/**/<skill>/SKILL.md` (any depth >= 3) is accepted, while
`skills/<skill>/SKILL.md` (depth 2) is rejected:

```ron
SkillLint(
    id: "skill_flat_layout",
    human_readable_name: "SKILL.md at wrong nesting depth",
    description: "A SKILL.md was found too close to the skills root. Skills must be at least depth 3: skills/<folder>/.../<skill-name>/SKILL.md",
    lint_level: Deny,
    reference_link: Some("https://agentskills.io/specification#directory-structure"),
    query: r#"
    {
        DiscoveredSkillFile {
            depth @filter(op: "<", value: ["$min_depth"]) @output

            path @output
            parent_dir @output

            span_: span {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {
        "min_depth": 3,
    },
    error_message: "A SKILL.md file was found too close to the skills root. Skills must be nested at least 3 levels deep: skills/<folder>/.../<skill-name>/SKILL.md.",
    per_result_error_template: Some("{{path}} is at depth {{depth}}, minimum is 3. Skills must be nested: skills/<folder>/.../<skill-name>/SKILL.md"),
)
```

### 4.5 `skill_duplicate_name.ron`

Uses the `all_other_skills` edge on `Skill` with `@tag` + `@filter` — no pre-computed
cross-skill type needed. Any cross-skill comparison (names, descriptions, licenses) can
be expressed the same way:

```ron
SkillLint(
    id: "skill_duplicate_name",
    human_readable_name: "duplicate skill name",
    description: "Two or more skills share the same `name` value in their frontmatter.",
    lint_level: Deny,
    reference_link: Some("https://agentskills.io/specification#name-field"),
    query: r#"
    {
        Skill {
            name @filter(op: "is_not_null")
                 @tag(name: "my_name")
                 @output

            skill_file_path @output(name: "this_path")

            all_other_skills {
                name @filter(op: "=", value: ["%my_name"])
                skill_file_path @output(name: "duplicate_path")
                group_folder @output(name: "duplicate_group")

                span_: span {
                    filename @output
                    begin_line @output
                    end_line @output
                }
            }
        }
    }"#,
    arguments: {},
    error_message: "Multiple skills share the same `name` value. Skill names must be unique across the entire repository.",
    per_result_error_template: Some("skill '{{this_path}}' has duplicate name '{{name}}' — also used by {{duplicate_path}}"),
)
```

### 4.6 `skill_mixed_script_languages.ron`

Uses the generic `SubDir` type. The "what counts as a script language" logic is now in
the query — filter out data files, then check unique extension count:

```ron
SkillLint(
    id: "skill_mixed_script_languages",
    human_readable_name: "scripts/ directory uses multiple languages",
    description: "A skill's scripts/ directory contains scripts in more than one programming language.",
    lint_level: Deny,
    reference_link: None,
    query: r#"
    {
        Skill {
            skill_file_path @output
            folder_name @output

            sub_dir @filter(op: "name", value: ["$scripts_dir_name"]) {
                unique_extensions @output(name: "script_extensions")

                file @fold
                     @transform(op: "count")
                     @filter(op: ">", value: ["$zero"]) {
                    is_data_file @filter(op: "=", value: ["$false"])
                }

                path @output(name: "scripts_path")
            }

            span_: span {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {
        "scripts_dir_name": "scripts",
        "zero": 0,
        "false": false,
    },
    error_message: "A skill's scripts/ directory contains scripts written in multiple programming languages. Use a single language per skill.",
    per_result_error_template: Some("{{scripts_path}} contains multiple script languages: {{script_extensions}}"),
)
```

### 4.7 `skill_description_too_short.ron`

```ron
SkillLint(
    id: "skill_description_too_short",
    human_readable_name: "skill description is too short",
    description: "The `description` field should be at least 20 words to adequately describe what the skill does and when to use it.",
    lint_level: Warn,
    reference_link: Some("https://agentskills.io/specification#description-field"),
    query: r#"
    {
        Skill {
            has_frontmatter @filter(op: "=", value: ["$true"])
            description @filter(op: "is_not_null") @output
            description_word_count @filter(op: "<", value: ["$min_words"]) @output

            skill_file_path @output

            span_: frontmatter_span @optional {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {
        "true": true,
        "min_words": 20,
    },
    error_message: "The `description` field is too short. It should be at least 20 words and describe both what the skill does and when an agent should activate it.",
    per_result_error_template: Some("{{skill_file_path}} description is only {{description_word_count}} words (minimum 20)"),
)
```

### 4.8 `skill_body_too_long.ron`

```ron
SkillLint(
    id: "skill_body_too_long",
    human_readable_name: "SKILL.md body exceeds 500 lines",
    description: "The SKILL.md body (excluding frontmatter) exceeds the recommended 500-line limit.",
    lint_level: Warn,
    reference_link: Some("https://agentskills.io/specification#progressive-disclosure"),
    query: r#"
    {
        Skill {
            body_line_count @filter(op: ">", value: ["$max_lines"]) @output

            skill_file_path @output

            span_: span {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {
        "max_lines": 500,
    },
    error_message: "SKILL.md body exceeds the recommended 500-line limit. Move detailed reference material to the references/ subdirectory.",
    per_result_error_template: Some("{{skill_file_path}} body is {{body_line_count}} lines (recommended maximum: 500)"),
)
```

### 4.9 `skill_missing_examples_section.ron`

```ron
SkillLint(
    id: "skill_missing_examples_section",
    human_readable_name: "SKILL.md missing Examples section",
    description: "The SKILL.md body should contain an '## Examples' section.",
    lint_level: Warn,
    reference_link: None,
    query: r#"
    {
        Skill {
            has_frontmatter @filter(op: "=", value: ["$true"])
            body_line_count @filter(op: ">", value: ["$zero"])

            section @fold
                    @transform(op: "count")
                    @filter(op: "=", value: ["$zero"]) {
                heading @filter(op: "regex", value: ["$examples_pattern"])
            }

            skill_file_path @output

            span_: span {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {
        "true": true,
        "zero": 0,
        "examples_pattern": "(?i)^examples?$",
    },
    error_message: "SKILL.md is missing an 'Examples' section. Include concrete usage examples to help agents understand the skill.",
    per_result_error_template: Some("{{skill_file_path}} has no '## Examples' section"),
)
```

### 4.10 `skill_name_invalid_format.ron`

```ron
SkillLint(
    id: "skill_name_invalid_format",
    human_readable_name: "skill name is not valid kebab-case",
    description: "The frontmatter `name` must be lowercase kebab-case: only lowercase letters, numbers, and single hyphens.",
    lint_level: Deny,
    reference_link: Some("https://agentskills.io/specification#name-field"),
    query: r#"
    {
        Skill {
            has_frontmatter @filter(op: "=", value: ["$true"])
            name @filter(op: "is_not_null")
                 @filter(op: "not_regex", value: ["$valid_name_pattern"])
                 @output

            skill_file_path @output

            span_: frontmatter_span @optional {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {
        "true": true,
        "valid_name_pattern": "^[a-z0-9]([a-z0-9-]*[a-z0-9])?$",
    },
    error_message: "The `name` field must be valid kebab-case: lowercase letters, numbers, and hyphens. Must not start or end with a hyphen, and must not contain consecutive hyphens.",
    per_result_error_template: Some("{{skill_file_path}} has invalid name '{{name}}'"),
)
```

## 5. Lint struct (Rust, mirrors `SemverQuery`)

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
```

## 6. Adapter implementation

### 6.1 Vertex enum

The adapter maps schema types to a Rust enum. Note the reduced variant count (10 vs. 14)
thanks to the generic `SubDir`/`SubDirFile` types and the removal of `DuplicateSkillName`:

```rust
#[derive(Debug, Clone)]
pub enum Vertex {
    Skill(Arc<SkillData>),
    GroupFolder(Arc<GroupFolderData>),
    DiscoveredSkillFile(Arc<DiscoveredSkillFileData>),
    Section(Arc<SectionData>),
    CodeBlock(Arc<CodeBlockData>),
    SubDir(Arc<SubDirData>),
    SubDirFile(Arc<SubDirFileData>),
    MetadataEntry(Arc<MetadataEntryData>),
    Span(Arc<SpanData>),
}
```

### 6.2 Data loading pipeline

```text
skills/ directory on disk
    → walk_skills_directory()
    → Vec<DiscoveredSkillFileData>     (ALL SKILL.md files found at any depth)
    → partition into:
        Vec<SkillData>                 (valid location, parsed)
        Vec<GroupFolderData>           (group directories)
        → for each valid SKILL.md:
            parse_frontmatter() → FrontmatterData (name, description, metadata, ...)
            parse_markdown_body() → Vec<SectionData>, Vec<CodeBlockData>
            scan_subdirs() → Vec<SubDirData> (scripts/, references/, assets/, any others)
    → SkillsAdapter::new(all_data)
```

### 6.3 Key crate dependencies

| Crate | Purpose |
|-------|---------|
| `trustfall` | Query engine |
| `trustfall_core` | Adapter trait (`BasicAdapter`) |
| `trustfall_stubgen` | Generate adapter skeleton from schema (one-time dev tool) |
| `serde`, `serde_yaml` | YAML frontmatter parsing |
| `ron` | Lint definition parsing |
| `pulldown-cmark` | Markdown parsing (headings, code blocks, sections) |
| `clap` | CLI argument parsing |
| `handlebars` | Error message templating |
| `toml` | Config file parsing (`.skill-validator.toml`) |
| `fs_err` | Better filesystem error messages |
| `walkdir` | Recursive directory traversal |
| `miette` or `ariadne` | Rich diagnostic output |

### 6.4 BasicAdapter trait implementation

The adapter implements four methods from `trustfall_core::interpreter::basic_adapter::BasicAdapter`:

1. **`resolve_starting_vertices`** — handles `Skill`, `GroupFolder`, and
   `DiscoveredSkillFile` entry points
2. **`resolve_property`** — maps schema properties to data fields on each `Vertex` variant
3. **`resolve_neighbors`** — handles edges like `Skill.section`, `Skill.sub_dir`,
   `Skill.all_other_skills`, `SubDir.file`, `Section.code_block`,
   `DiscoveredSkillFile.skill`, etc.
4. **`resolve_coercion`** — not needed (no interfaces/unions in the schema)

The `all_other_skills` edge is implemented by filtering the adapter's `Vec<SkillData>` to
exclude the current skill. This is O(n) per skill and O(n²) overall, which is acceptable
for skill repos (< 500 skills). If performance becomes a concern, the adapter can add
internal indexes without changing the schema.

Use `trustfall_stubgen` to generate the initial skeleton from the `.graphql` schema, then fill
in the implementations.

## 7. CLI design

```text
skill-validator [OPTIONS] [SKILLS_DIR]

Arguments:
  [SKILLS_DIR]    Path to the skills directory (overrides config, default: ./skills)

Options:
  -c, --config <PATH>    Path to .skill-validator.toml [default: ./.skill-validator.toml]
  -f, --format <FMT>     Output format: human, json, github-actions [default: human]
  -l, --lint <ID>         Run only specific lint(s) (repeatable)
      --deny <ID>         Override lint level to Deny (repeatable)
      --warn <ID>         Override lint level to Warn (repeatable)
      --allow <ID>        Override lint level to Allow (repeatable)
      --list-lints        List all available lints and exit
      --explain <ID>      Show detailed explanation for a lint
      --color <WHEN>      Color output: auto, always, never [default: auto]
  -q, --quiet             Only show errors, suppress warnings
  -v, --verbose           Show detailed diagnostic info
  -h, --help              Print help
  -V, --version           Print version
```

### GitHub Actions output format

When `--format github-actions` is used (auto-detected via `$GITHUB_ACTIONS` env var),
diagnostics use `::error::` and `::warning::` annotations for inline PR comments.

## 8. GitHub Action (`skill_validator/action.yml`)

Lives at `skill_validator/action.yml` — consumers reference it as
`uses: elastic/clients-team-automations/skill_validator@<ref>`.

The existing actions in this repo use composite steps that install Node.js and run
`npm ci && npm run build && node dist/index.js`. Since skill-validator is a Rust binary,
the composite action instead downloads a pre-built release binary. A `version` input
allows pinning to a specific release tag.

```yaml
name: 'Validate Agent Skills'
description: 'Lint and validate Agent Skills repository structure and content'

inputs:
  skills-dir:
    description: 'Path to the skills directory'
    required: false
    default: 'skills'
  config:
    description: 'Path to .skill-validator.toml'
    required: false
    default: '.skill-validator.toml'
  version:
    description: 'skill-validator release version to download (e.g. "v0.1.0"). Defaults to latest.'
    required: false
    default: 'latest'
  extra-args:
    description: 'Additional CLI arguments'
    required: false
    default: ''

outputs:
  exit-code:
    description: 'Exit code of the skill-validator run (0 = pass, 1 = lint failures found)'
    value: ${{ steps.validate.outputs.exit-code }}

runs:
  using: 'composite'
  steps:
    - name: 'Determine download URL'
      id: url
      shell: 'bash'
      run: |
        VERSION="${{ inputs.version }}"
        if [ "$VERSION" = "latest" ]; then
          URL="https://github.com/elastic/clients-team-automations/releases/latest/download"
        else
          URL="https://github.com/elastic/clients-team-automations/releases/download/${VERSION}"
        fi
        OS=$(uname -s | tr '[:upper:]' '[:lower:]')
        ARCH=$(uname -m)
        echo "url=${URL}/skill-validator-${OS}-${ARCH}.tar.gz" >> "$GITHUB_OUTPUT"

    - name: 'Install skill-validator'
      shell: 'bash'
      run: |
        curl -fsSL "${{ steps.url.outputs.url }}" | tar xz -C /usr/local/bin

    - name: 'Run validation'
      id: validate
      shell: 'bash'
      run: |
        skill-validator \
          --format github-actions \
          --config "${{ inputs.config }}" \
          ${{ inputs.extra-args }} \
          "${{ inputs.skills-dir }}"
        echo "exit-code=$?" >> "$GITHUB_OUTPUT"
```

### Consumer usage

Consuming repos reference the action via the subdirectory path (same pattern as the
other actions in this repo):

```yaml
# In a consumer repo's .github/workflows/validate-skills.yml
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: elastic/clients-team-automations/skill_validator@main
        with:
          skills-dir: skills
```

## 9. Pre-commit hook (`.pre-commit-hooks.yaml`)

The `.pre-commit-hooks.yaml` file **must live at the repo root** — pre-commit looks for it
there regardless of mono-repo structure. This is the one file that lives outside the
`skill_validator/` directory.

Since this is a mono-repo and `language: rust` would require pre-commit to build the entire
Cargo project, we use `language: system` and expect the binary to be pre-installed (via
`cargo install` or a release binary). Alternatively, `additional_dependencies` could be used
with a `language: rust` hook, but that adds significant build time.

```yaml
# .pre-commit-hooks.yaml  (repo root — not inside skill_validator/)
- id: validate-skills
  name: Validate Agent Skills
  entry: skill-validator
  language: system
  files: ^skills/
  pass_filenames: false
  description: Validate skill folder structure, frontmatter, and content conventions.
```

Consumer repos add to their `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/elastic/clients-team-automations
    rev: v0.1.0
    hooks:
      - id: validate-skills
        additional_dependencies: []  # requires skill-validator binary on PATH
```

> **Note:** The repo root `.pre-commit-hooks.yaml` is shared by all actions in this
> mono-repo. If other actions later add pre-commit hooks, they are appended to the same file.

## 9.1. CI workflow (`.github/workflows/skill-validator.yml`)

Each action in this repo has a corresponding workflow file for CI testing. Following that
pattern, the skill validator gets `.github/workflows/skill-validator.yml`.

This workflow runs `cargo test` and `cargo clippy` on PRs that touch `skill_validator/`
files. It also builds release binaries on tagged releases.

```yaml
name: 'Skill Validator'

on:
  push:
    branches: [main]
    paths: ['skill_validator/**']
  pull_request:
    paths: ['skill_validator/**']

defaults:
  run:
    working-directory: skill_validator

jobs:
  test:
    name: 'Test'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: skill_validator
      - run: cargo test --all-features
      - run: cargo clippy -- -D warnings

  release:
    name: 'Build Release Binaries'
    if: startsWith(github.ref, 'refs/tags/skill-validator-v')
    needs: [test]
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --release --target ${{ matrix.target }}
      - name: Package binary
        run: |
          tar czf skill-validator-${{ matrix.target }}.tar.gz \
            -C target/${{ matrix.target }}/release skill-validator
      - uses: softprops/action-gh-release@v2
        with:
          files: skill_validator/skill-validator-${{ matrix.target }}.tar.gz
```

> **Release tagging convention:** Since this is a mono-repo, release tags are prefixed
> with the action name: `skill-validator-v0.1.0`. This avoids collisions with tags from
> other actions and matches the `if: startsWith(...)` filter in the release job.

## 10. Full lint inventory

These lints cover everything in the current `validate-skills.sh` plus additional checks
from the agentskills.io specification and `CONTRIBUTING.md`.

### Deny-level (errors)

| Lint ID | Replaces bash check | Description |
|---------|-------------------|-------------|
| `skill_flat_layout` | Depth-1 rejection loop | SKILL.md found at wrong nesting depth |
| `skill_missing_frontmatter` | Empty frontmatter check | No YAML frontmatter delimiters |
| `skill_missing_name` | `fm_name` empty check | Frontmatter missing `name` field |
| `skill_missing_description` | `fm_desc` empty check | Frontmatter missing `description` field |
| `skill_name_missing_group_prefix` | Name vs expected check | `name` doesn't start with group folder |
| `skill_name_missing_folder_suffix` | Name vs expected check | `name` doesn't end with skill folder |
| `skill_name_invalid_format` | — (new) | `name` is not valid kebab-case per spec |
| `skill_name_consecutive_hyphens` | — (new) | `name` contains `--` |
| `skill_name_too_long` | — (new) | `name` exceeds 64 characters |
| `skill_duplicate_name` | `seen_names` check | Two skills share the same name |
| `skill_mixed_script_languages` | Extension uniqueness check | `scripts/` uses multiple languages |
| `skill_description_too_long` | — (new) | `description` exceeds 1024 characters |

### Warn-level (warnings)

| Lint ID | Description |
|---------|-------------|
| `skill_description_too_short` | Description under 20 words |
| `skill_body_too_long` | Body exceeds 500 lines |
| `skill_missing_examples_section` | No `## Examples` section |
| `skill_missing_guidelines_section` | No `## Guidelines` section |
| `skill_code_block_missing_language` | Fenced code block without language tag |
| `skill_missing_title_heading` | No `# Title` heading in body |

### Allow-level (opt-in)

| Lint ID | Description |
|---------|-------------|
| `skill_has_no_scripts` | Skill has no `scripts/` directory |
| `skill_has_no_references` | Skill has no `references/` directory |

## 11. Testing strategy

Follow the `cargo-semver-checks` pattern:

1. **Test skills** in `skill_validator/test_crates/` — each is a minimal skill directory exercising one lint
2. **Snapshot tests** using `insta` — expected outputs in `skill_validator/test_outputs/`
3. **Run all lints against all test skills** — one snapshot per lint
4. **CI runs `cargo test`** in the `skill_validator/` working directory (see section 9.1) — snapshots are checked in

```text
skill_validator/test_crates/
├── valid_skill/
│   └── skills/
│       └── test/
│           └── valid-skill/
│               ├── SKILL.md          # Valid skill with all fields
│               └── scripts/
│                   └── helper.py
├── missing_name/
│   └── skills/
│       └── test/
│           └── missing-name/
│               └── SKILL.md          # Missing name field
├── flat_layout/
│   └── skills/
│       └── flat-skill/
│           └── SKILL.md              # At wrong depth
├── mixed_languages/
│   └── skills/
│       └── test/
│           └── mixed-langs/
│               ├── SKILL.md
│               └── scripts/
│                   ├── a.py
│                   └── b.sh          # Two languages
└── ...
```

### `.gitignore` for the Rust crate

The repo root `.gitignore` is tuned for Node.js projects (`node_modules`, `dist`, `*.d.ts`,
etc.). Rather than modifying the shared ignore file, `skill_validator/` gets its own:

```gitignore
# skill_validator/.gitignore
/target/
```

This keeps the Rust build artifacts out of git without affecting sibling actions.

## 12. Implementation phases

### Phase 1: Core engine (MVP)

- [ ] Create `skill_validator/` directory with `Cargo.toml` and `skill_validator/.gitignore`
- [ ] Define the GraphQL schema (`skill_validator/skills_schema.graphql`)
- [ ] Use `trustfall_stubgen` to generate adapter skeleton
- [ ] Implement config loading (`.skill-validator.toml` with defaults)
- [ ] Implement data loading (directory walking, frontmatter parsing, markdown parsing)
- [ ] Implement the adapter (`resolve_starting_vertices`, `resolve_property`, `resolve_neighbors`)
- [ ] Implement `all_other_skills` edge for cross-skill queries
- [ ] Implement `DiscoveredSkillFile` entry point (all SKILL.md files at any depth)
- [ ] Implement lint loading from `.ron` files
- [ ] Implement lint execution engine (with config-driven lint level merging)
- [ ] Port all `validate-skills.sh` checks as `.ron` lints
- [ ] Add CLI with `clap` (including `--config`)
- [ ] Add `--format github-actions` output

### Phase 2: Query mode & testing

- [ ] Add `skill-validator query` subcommand (thin wrapper over `trustfall::execute_query`)
- [ ] Add `--schema` flag to print the GraphQL schema
- [ ] Add query output formatters: table, json, csv
- [ ] Ship `skills_schema.graphql` as a published file for editor LSP integration
- [ ] Create test skill directories in `skill_validator/test_crates/`
- [ ] Add snapshot tests with `insta`
- [ ] Test config file loading (custom name patterns, lint level overrides)
- [ ] Add `--list-lints` and `--explain` commands
- [ ] Add lint level overrides (`--deny`, `--warn`, `--allow`)
- [ ] Rich diagnostic output with `miette` or `ariadne`
- [ ] Error message templating with `handlebars`

### Phase 3: Distribution (mono-repo aware)

- [ ] Create `skill_validator/action.yml` composite action (downloads pre-built binary)
- [ ] Create `.pre-commit-hooks.yaml` at repo root (system language, expects binary on PATH)
- [ ] Add `.github/workflows/skill-validator.yml` for CI (test on PR, build on tag)
- [ ] Set up release workflow with prefixed tags (`skill-validator-v0.1.0`)
- [ ] Build and upload release binaries for Linux x86_64/aarch64 and macOS x86_64/aarch64
- [ ] Add `skill_validator/README.md` documenting the action, CLI, and pre-commit usage
- [ ] Update `agent-skills-sandbox` CI to use `elastic/clients-team-automations/skill_validator@main`
- [ ] Update `agent-skills-sandbox` `.pre-commit-config.yaml`

### Phase 4: Extended lints

- [ ] Add spec-compliance lints (name format, description length, etc.)
- [ ] Add content-quality lints (code block language tags, heading structure)
- [ ] Add support for custom user-defined lints (load from a config directory)
- [ ] Consider a `--fix` mode for auto-fixable issues

## 13. Advantages over the bash script

| Aspect | `validate-skills.sh` | `skill-validator` |
|--------|----------------------|-------------------|
| Adding a check | Edit bash, handle edge cases | Add a `.ron` file |
| Error messages | Hand-crafted in bash | Templated, consistent |
| Testing | None | Snapshot tests per lint |
| Extensibility | Requires bash knowledge | Declarative queries |
| Performance | Sequential `find`/`awk` | Parallel, lazy evaluation |
| Output formats | GitHub Actions only | Human, JSON, GH Actions |
| Maintenance | Fragile string processing | Type-safe schema + queries |
| User customization | Fork the script | `--deny`/`--warn`/`--allow` per lint |
| Ad-hoc exploration | Not possible | `skill-validator query` — arbitrary Trustfall queries |
| Lint authoring | Write bash | Write a query, save as `.ron` (explore → discover → codify flywheel) |
| Distribution | Copy the script | Binary, Action (`elastic/clients-team-automations/skill_validator`), pre-commit hook |

## 14. Ad-hoc query mode (`skill-validator query`)

The schema and adapter exist to power lints — but they also constitute a fully queryable
knowledge graph of the skills catalog. Exposing this as an interactive query interface costs
almost nothing (the adapter is already built) and transforms the tool from a CI linter into a
**skills intelligence platform**.

### Why this matters

Every other linting tool (shellcheck, eslint, clippy, vale) is a black box: it runs its
checks and exits. You cannot ask it an arbitrary question about your codebase. Trustfall
makes this possible for free — the same adapter that answers lint queries can answer *any*
query expressible over the schema.

This creates a **flywheel**:

```text
explore (ad-hoc query)
  → discover a pattern ("17 skills have no Examples section")
    → codify it (save the query as a .ron lint)
      → enforce it (lint catches future violations in CI)
        → explore further ...
```

Lints stop being a fixed checklist and start being a living, growing set of institutional
knowledge — authored by anyone who can write a Trustfall query, with no Rust code changes.

### CLI subcommand

```text
skill-validator query [OPTIONS] [SKILLS_DIR]

Run an ad-hoc Trustfall query against the skills repository.

Arguments:
  [SKILLS_DIR]    Path to the skills directory (default from config)

Options:
  -q, --query <QUERY>     Trustfall query string (reads from stdin if omitted)
  -a, --args <JSON>       Query arguments as JSON object [default: {}]
  -f, --format <FMT>      Output format: table, json, csv [default: table]
  -c, --config <PATH>     Path to .skill-validator.toml
      --schema            Print the full GraphQL schema and exit
  -h, --help              Print help
```

### Example queries

**Find all skills without an Examples section:**

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

This is exactly the query inside `skill_missing_examples_section.ron`. Someone exploring
their repo with `query` discovers this pattern, then saves it as a lint. The flywheel turns.

**Skills per group folder (catalog overview):**

```bash
skill-validator query -q '{
    GroupFolder {
        name @output
        skill_count @output
    }
}'
```

**Which skills have the longest body (candidates for splitting into references/):**

```bash
skill-validator query -q '{
    Skill {
        skill_file_path @output
        body_line_count @filter(op: ">", value: ["$threshold"]) @output
        description @output
    }
}' -a '{"threshold": 200}'
```

**Skills with code blocks missing language tags:**

```bash
skill-validator query -q '{
    Skill {
        skill_file_path @output
        section {
            heading @output
            code_block {
                has_language_tag @filter(op: "=", value: ["$false"])
                line_number @output
            }
        }
    }
}' -a '{"false": false}'
```

**Audit naming convention compliance without running all lints:**

```bash
skill-validator query -q '{
    Skill {
        name @filter(op: "is_not_null")
             @filter(op: "not_has_prefix", value: ["%group"])
             @output
        group_folder @tag(name: "group") @output
        folder_name @output
        skill_file_path @output
    }
}'
```

### Schema introspection (`--schema`)

`skill-validator query --schema` prints the full `.graphql` schema to stdout, making it
easy to pipe into editor integrations or documentation tools. Combined with a
[`.graphqlrc.yml`](https://the-guild.dev/graphql/config) pointing at the schema file, users
get LSP-powered autocomplete and validation while writing queries in their editor — the same
workflow [recommended for cargo-semver-checks](https://github.com/obi1kenobi/trustfall/discussions/679).

### Implementation cost

The query subcommand is a thin wrapper:

```rust
fn run_query(schema: &Schema, adapter: &SkillsAdapter, query: &str, args: BTreeMap<String, FieldValue>) {
    let results = trustfall::execute_query(schema, adapter.clone(), query, args)
        .expect("query execution failed");
    for result in results {
        // format and print each row
    }
}
```

The adapter, schema, and data loading pipeline are already built for linting. The query
subcommand reuses all of it. The incremental cost is: one `clap` subcommand, one output
formatter, and a handful of lines of glue.

### Publishing the schema

Ship `skills_schema.graphql` as a file inside the repo (and embed it in the binary via
`include_str!`). This serves triple duty:

1. **`trustfall_stubgen`** reads it to generate the adapter skeleton during development
2. **`skill-validator query --schema`** prints it for users
3. **Editor LSP** uses it for autocomplete via `.graphqlrc.yml`

### Future extensions

The query mode is also the natural foundation for:

- **`skill-validator report`** — pre-built queries that produce catalog-level summaries
  (skill count by group, coverage gaps, freshness metrics)
- **Piped composition** — `skill-validator query ... --format json | jq ...` for scripted
  workflows
- **Cross-adapter federation** — Trustfall supports composing multiple adapters into a single
  query. A future `--with-adapter elastic-docs` flag could enable queries that correlate
  skills with the actual Elastic documentation they reference, validating semantic correctness
  against the source of truth — something no other linter can do

## 15. Design decisions (resolved)

1. **Config file: yes.** Support a `.skill-validator.toml` for per-repo lint level overrides,
   custom lint directories, and naming convention configuration. See [section 16](#16-configuration-file).

2. **Cross-skill queries: via `all_other_skills` edge, not dedicated types.** Instead of
   pre-computed entry points like `DuplicateSkillName`, cross-skill comparisons are expressed
   using the `all_other_skills` edge with `@tag` + `@filter` in `.ron` queries. This means
   new cross-skill lints (duplicate names, overlapping descriptions, conflicting licenses)
   are purely declarative — no Rust code changes required. See the `skill_duplicate_name.ron`
   example in section 4.5.

3. **Generic filesystem types: `SubDir` + `SubDirFile` replace 6 specialized types.** The
   former `ScriptsDir`/`ReferencesDir`/`AssetsDir` and their file types were structurally
   identical. One generic pair covers all subdirectories. The `is_data_file` classification
   is config-driven (via `data_extensions` in `.skill-validator.toml`). Future subdirectories
   need zero schema changes.

4. **`DiscoveredSkillFile` instead of `InvalidSkillFile`.** The adapter emits ALL found
   SKILL.md files. The "what is invalid" policy lives in `.ron` queries (filter on
   `depth`), not baked into the adapter.

5. **Validate `references/` and `assets/` content: not in scope for now.** The schema models
   these directories via `SubDir` for structural checks (existence, file listing), but content
   validation (e.g., checking that referenced files exist) is deferred to a future phase.

6. **Python bindings: no.** The tool is Rust-only. Distribution as a compiled binary keeps
   CI fast and dependency-free.

7. **Naming convention: expressed in `.ron` queries.** The Elastic convention
   (`<group>-<skill-folder>`) is encoded as two lints (`skill_name_missing_group_prefix`
   and `skill_name_missing_folder_suffix`). Repos that don't use group prefixes disable
   the prefix lint via `[lints]` in `.skill-validator.toml`.

## 16. Configuration file

The optional `.skill-validator.toml` file lives at the repo root. When absent, all defaults
apply. The CLI flag `--config <path>` overrides the default location.

```toml
# .skill-validator.toml

# Path to the skills directory (default: "skills")
skills_dir = "skills"

# Per-lint level overrides. Keys are lint IDs, values are "deny", "warn", or "allow".
[lints]
skill_body_too_long = "deny"          # promote from warn to deny
skill_has_no_scripts = "warn"         # promote from allow to warn
skill_missing_guidelines_section = "allow"   # suppress this warning

# Additional directories to load custom .ron lint files from.
# Paths are relative to the repo root.
custom_lint_dirs = ["my-lints/"]

# File extensions considered "data files" (not scripts) in scripts/ directories.
# These are excluded when checking for mixed script languages.
# Default: ["txt", "md", "json", "yaml", "yml", "cfg", "ini", "toml", "env", "csv"]
data_extensions = ["txt", "md", "json", "yaml", "yml", "cfg", "ini", "toml", "env", "csv"]
```

### Config loading precedence (highest wins)

1. CLI flags (`--deny`, `--warn`, `--allow`, `--lint`)
2. `.skill-validator.toml` `[lints]` table
3. Built-in lint defaults from the `.ron` file's `lint_level` field

### Config struct (Rust)

```rust
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
}
```

### Impact on the adapter

The naming convention is expressed entirely in `.ron` queries using `@tag` +
`has_prefix`/`has_suffix` on `group_folder` and `folder_name`. The adapter provides
these raw properties; the convention is policy defined in the `.ron` files, not baked
into Rust. Repos that don't use group prefixes can disable `skill_name_missing_group_prefix`
via `[lints]` in the config file.

## 17. Cross-skill queries via `all_other_skills`

Instead of pre-computed entry points like the former `DuplicateSkillName`, all cross-skill
comparisons use the `all_other_skills` edge on `Skill`. This edge returns every other skill
in the repository, enabling arbitrary pairwise comparisons via `@tag` + `@filter`.

### How it works

```graphql
type Skill {
    # ... existing fields ...

    """
    All other skills in the repository (excluding this one).
    Enables cross-skill comparisons entirely within .ron queries.
    """
    all_other_skills: [Skill!]!
}
```

The adapter implements this by iterating `Vec<SkillData>` and filtering out the current
skill (by path). This is O(n) per skill, O(n²) overall — acceptable for skill repos.

### Example: duplicate name detection (pure query)

```ron
query: r#"
{
    Skill {
        name @filter(op: "is_not_null")
             @tag(name: "my_name")
             @output

        skill_file_path @output(name: "this_path")

        all_other_skills {
            name @filter(op: "=", value: ["%my_name"])
            skill_file_path @output(name: "duplicate_path")
        }
    }
}"#,
```

### Example: duplicate description detection (no Rust changes needed)

```ron
query: r#"
{
    Skill {
        description @filter(op: "is_not_null")
                    @tag(name: "my_desc")
                    @output

        skill_file_path @output(name: "this_path")

        all_other_skills {
            description @filter(op: "=", value: ["%my_desc"])
            skill_file_path @output(name: "duplicate_path")
        }
    }
}"#,
```

### Example: skills in same group with similar names (no Rust changes needed)

```ron
query: r#"
{
    Skill {
        group_folder @tag(name: "my_group") @output
        name @tag(name: "my_name") @output
        skill_file_path @output(name: "this_path")

        all_other_skills {
            group_folder @filter(op: "=", value: ["%my_group"])
            name @filter(op: "regex", value: ["%my_name"])
            skill_file_path @output(name: "similar_path")
        }
    }
}"#,
```

The key insight: every future cross-skill lint is just a new `.ron` file. The adapter
provides one generic edge, and `.ron` queries compose it with `@tag` + `@filter` to express
any comparison. No new Rust types, no new entry points, no recompilation.
