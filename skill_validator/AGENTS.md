# Writing Custom Lint Rules (`.ron` files)

Custom lint rules for the skill validator are defined as `.ron` (Rusty Object Notation) files. Each file contains a
single `SkillLint(...)` struct that declares a query over the skill data model. When the query returns results, the lint
fires.

## How Custom Lints Are Loaded

Place `.ron` files in any directory, then register that directory in `.skill-validator.toml`:

```toml
custom_lint_dirs = ["path/to/this/directory"]
```

Every `.ron` file in the listed directories is automatically loaded at runtime. No other registration is needed.

---

## SkillLint Struct Reference

Every `.ron` file must contain exactly one `SkillLint(...)` value. Here is the full set of fields:

```ron
SkillLint(
    // REQUIRED. Unique identifier for this lint. Use kebab-case.
    // Must be unique across all builtin and custom lints.
    id: "my-custom-lint",

    // REQUIRED. Short human-readable name shown in reports.
    human_readable_name: "description of what this lint checks",

    // REQUIRED. Longer explanation of the rule.
    description: "A detailed description of what this lint checks and why.",

    // REQUIRED. Default severity level. One of: Deny, Warn, Allow.
    //   Deny  = hard error, fails the check
    //   Warn  = warning, does not fail the check
    //   Allow = disabled by default, opt-in via config override
    lint_level: Deny,

    // OPTIONAL. A URL to relevant documentation or specification.
    // Use Some("https://...") or None.
    reference_link: None,

    // REQUIRED. The Trustfall query that finds violations.
    // Use a raw string: r#"..."#
    // Each row returned by this query is one lint violation.
    query: r#"
    {
        Skill {
            ...
        }
    }"#,

    // REQUIRED (may be empty). A map of named arguments used in the query.
    // Keys are plain strings. Values can be strings, integers, booleans,
    // floats, or lists. Reference them in queries as $key_name.
    arguments: {
        "my_arg": "some_value",
    },

    // REQUIRED. The error message shown when this lint fires.
    error_message: "What went wrong and how to fix it.",

    // OPTIONAL. A Handlebars template for per-result detail messages.
    // Use Some("...") or None. Placeholders: {{field_name}}
    // Available variables: all @output fields from the query + all argument keys.
    per_result_error_template: Some("{{skill_file_path}} has problem: {{some_field}}"),
)
```

### Field details

| Field                       | Type   | Required | Description                                                                               |
| --------------------------- | ------ | -------- | ----------------------------------------------------------------------------------------- |
| `id`                        | String | Yes      | Unique kebab-case identifier. Must not collide with builtin lint IDs.                     |
| `human_readable_name`       | String | Yes      | Short label for the lint (shown in summaries).                                            |
| `description`               | String | Yes      | Explains what the lint checks and why it matters.                                         |
| `lint_level`                | Enum   | Yes      | `Deny`, `Warn`, or `Allow`. Can be overridden in `.skill-validator.toml` under `[lints]`. |
| `reference_link`            | Option | No       | `Some("url")` or `None`.                                                                  |
| `query`                     | String | Yes      | A Trustfall query (see below). Wrap in `r#"..."#`.                                        |
| `arguments`                 | Map    | Yes      | `{}` if the query takes no arguments. Keys are strings, values are typed literals.        |
| `error_message`             | String | Yes      | Static message shown for every violation found by this lint.                              |
| `per_result_error_template` | Option | No       | `Some("template")` or `None`. Handlebars template rendered per result row.                |

### Lint level overrides

Users can override lint levels in `.skill-validator.toml`:

```toml
[lints]
my-custom-lint = "warn"    # downgrade from Deny to Warn
skill_has_no_scripts = "deny"  # upgrade from Allow to Deny
```

---

## Query Language (Trustfall)

Queries use Trustfall, a GraphQL-dialect query language. A lint's query describes a pattern of data that constitutes a
violation. Every row the query returns becomes one lint finding.

### Entry Points

Every query starts from one of these root types:

| Entry Point           | Description                                                                                     |
| --------------------- | ----------------------------------------------------------------------------------------------- |
| `Skill`               | All valid skills (SKILL.md at correct nesting depth). Most lints use this.                      |
| `GroupFolder`         | All group folders directly under the skills root.                                               |
| `DiscoveredSkillFile` | Every SKILL.md found anywhere, including invalid locations. Use for structural checks.          |
| `DiscoveredDirectory` | All directories at skill depth (>= 2 levels under skills root), with or without a SKILL.md.    |
| `GitHubOrg`           | The configured GitHub organization. Provides team data for validation. Requires a GitHub token. |

### Directives

#### `@output` -- Include a field in results

Marks a property to be included in the query output. Only `@output` fields are available in error templates.

```graphql
name @output                        # output with the field's own name
name @output(name: "skill_name")    # output with an alias
```

#### `@filter` -- Filter/match values

Filters rows based on a condition. The `value` array contains either `$argument` references or `%tag` references.

```graphql
name @filter(op: "=", value: ["$expected_name"])
name @filter(op: "regex", value: ["$pattern"])
description @filter(op: "is_not_null")
```

#### `@tag` -- Capture a value for cross-reference

Saves a field's value so it can be referenced later in a `@filter` using `%tag_name`.

```graphql
name @tag(name: "my_name") @output
# ... later, in a nested context:
name @filter(op: "=", value: ["%my_name"])
```

#### `@fold` + `@transform` -- Aggregate over a list

Collapses a list edge into a single value (like SQL aggregation). Commonly used with `@transform(op: "count")` to count
items and then `@filter` on the count.

```graphql
section @fold
        @transform(op: "count")
        @filter(op: "=", value: ["$zero"]) {
    heading @filter(op: "regex", value: ["$pattern"])
}
```

This means: "count sections whose heading matches the pattern, and keep only skills where that count equals zero" --
that is, skills missing the section.

#### `@optional` -- Nullable edge traversal

Allows traversal of an edge that might not exist. The query still returns results even if the edge has no target.

```graphql
span_: frontmatter_span @optional {
    filename @output
    begin_line @output
    end_line @output
}
```

### Filter Operators

| Operator         | Description                | Example                                          |
| ---------------- | -------------------------- | ------------------------------------------------ |
| `=`              | Equals                     | `@filter(op: "=", value: ["$val"])`              |
| `!=`             | Not equals                 | `@filter(op: "!=", value: ["$val"])`             |
| `<`              | Less than                  | `@filter(op: "<", value: ["$max"])`              |
| `>`              | Greater than               | `@filter(op: ">", value: ["$min"])`              |
| `<=`             | Less than or equal         | `@filter(op: "<=", value: ["$max"])`             |
| `>=`             | Greater than or equal      | `@filter(op: ">=", value: ["$min"])`             |
| `regex`          | Matches regex              | `@filter(op: "regex", value: ["$pattern"])`      |
| `not_regex`      | Does not match regex       | `@filter(op: "not_regex", value: ["$pattern"])`  |
| `is_null`        | Value is null              | `@filter(op: "is_null")`                         |
| `is_not_null`    | Value is not null          | `@filter(op: "is_not_null")`                     |
| `has_prefix`     | String starts with         | `@filter(op: "has_prefix", value: ["%tag"])`     |
| `not_has_prefix` | String does not start with | `@filter(op: "not_has_prefix", value: ["%tag"])` |
| `has_suffix`     | String ends with           | `@filter(op: "has_suffix", value: ["%tag"])`     |
| `not_has_suffix` | String does not end with   | `@filter(op: "not_has_suffix", value: ["%tag"])` |

Note: `is_null` and `is_not_null` take no `value` parameter. All other operators require `value`.

### Variable Prefixes

- `$name` -- references a key in the `arguments` map.
- `%name` -- references a value captured earlier with `@tag(name: "name")`.

---

## Complete Data Schema

This is the full GraphQL schema describing every type, property, and edge available to queries.

```graphql
schema {
  query: RootQuery
}

type RootQuery {
  Skill: [Skill!]!
  GroupFolder: [GroupFolder!]!
  DiscoveredSkillFile: [DiscoveredSkillFile!]!
  DiscoveredDirectory: [DiscoveredDirectory!]!
  GitHubOrg: GitHubOrg!
}

type Skill {
  # Identity & location
  folder_name: String! # Directory name of the skill folder (e.g. "query-authoring")
  group_folder: String! # Group folder name (e.g. "elasticsearch")
  path: String! # Relative path to skill directory from repo root
  skill_file_path: String! # Relative path to SKILL.md from repo root
  depth: Int! # Nesting depth under skills root
  # Frontmatter properties
  has_frontmatter: Boolean! # Whether SKILL.md has valid YAML frontmatter
  raw_frontmatter: String # Raw YAML frontmatter text (nullable)
  name: String # name field from frontmatter (nullable)
  description: String # description field from frontmatter (nullable)
  license: String # license field from frontmatter (nullable)
  compatibility: String # compatibility field from frontmatter (nullable)
  allowed_tools: String # allowed-tools field from frontmatter (nullable)
  description_length: Int! # Character count of description
  description_word_count: Int! # Word count of description
  # Body properties
  total_line_count: Int! # Total lines in SKILL.md
  body_line_count: Int! # Lines in body (excluding frontmatter)
  has_title_heading: Boolean! # Whether body starts with # heading
  title_heading: String # Text of first # heading (nullable)
  # Edges (nested objects)
  metadata: [MetadataEntry!]! # Frontmatter key-value pairs
  section: [Section!]! # All ## headings in the body
  sub_dir: [SubDir!]! # Subdirectories (scripts/, references/, etc.)
  root_file: [SubDirFile!]! # Non-SKILL.md files directly in the skill folder
  referenced_path: [ReferencedPath!]! # Local path references from SKILL.md links/images
  all_other_skills: [Skill!]! # Every other skill in the repo (for cross-checks)
  span: Span! # Source location of the SKILL.md file
  frontmatter_span: Span # Source location of frontmatter block (nullable)
  name_span: Span # Source location of the `name` field line in frontmatter (nullable)
  description_span: Span # Source location of the `description` field line in frontmatter (nullable)
  compatibility_span: Span # Source location of the `compatibility` field line in frontmatter (nullable)
  frontmatter_end_span: Span # Single-line span at the closing `---` of frontmatter (nullable)
  github_org: GitHubOrg! # The configured GitHub org (for team validation, etc.)
}

type GroupFolder {
  name: String! # Folder name
  path: String! # Relative path from repo root
  skill: [Skill!]! # Skills in this group
  skill_count: Int! # Count of skills
}

type DiscoveredSkillFile {
  path: String! # Relative path from repo root
  parent_dir: String! # Parent directory name
  depth: Int! # Nesting depth under skills root
  skill: Skill # Resolved Skill if at valid location (nullable)
  span: Span! # Source location
}

type DiscoveredDirectory {
  name: String! # Directory name (e.g. "query-authoring")
  path: String! # Relative path from repo root
  depth: Int! # Depth under skills root (skill dirs are typically at depth 2)
  has_skill_file: Boolean! # Whether this directory contains a SKILL.md
  file_count: Int! # Count of non-SKILL.md files directly in this directory
  file: [SubDirFile!]! # Non-SKILL.md files directly in this directory
  skill: Skill # Resolved Skill if directory contains a valid skill (nullable)
  span: Span! # Source location (synthetic, points to the directory path)
}

type Section {
  level: Int! # Heading level (1 = #, 2 = ##, etc.)
  heading: String! # Heading text without the # prefix
  line_number: Int! # Line number where heading appears
  content: String! # Full text under this heading
  content_line_count: Int! # Line count of section content
  code_block: [CodeBlock!]! # Code blocks within this section
}

type CodeBlock {
  language: String # Language tag (e.g. "json", "bash") (nullable)
  has_language_tag: Boolean! # Whether a language tag is present
  line_number: Int! # Line where code block starts
  content: String! # Code block content
}

type SubDir {
  name: String! # Directory name (e.g. "scripts", "references")
  path: String! # Relative path from repo root
  file: [SubDirFile!]! # Files in this subdirectory
  file_count: Int! # Total file count
  unique_extensions: [String!]! # Unique file extensions (non-data files only)
  unique_extension_count: Int! # Count of unique non-data extensions
}

type SubDirFile {
  name: String! # File name
  extension: String! # File extension
  path: String! # Relative path from repo root
  is_data_file: Boolean! # Whether classified as data (json, yaml, csv, etc.)
  content: String # Text content of the file (null if binary/too large)
  referenced_path: [ReferencedPath!]! # Local path references extracted from file content
}

type ReferencedPath {
  raw_path: String! # Path string as written in source (e.g. "../../shared/es-client.js")
  resolved_path: String # Normalized repo-root-relative path (nullable if unresolvable)
  kind: String! # Reference type: markdown_link, markdown_image, js_import, js_require, js_dynamic_import, python_relative_import, shell_source
  line_number: Int! # 1-based line where the reference appears
  span: Span! # Source location pointing to the exact line in the source file where this reference appears
}

type MetadataEntry {
  key: String! # Metadata key
  value: String! # Metadata value (stringified for mappings)
  children: [MetadataEntry!]! # Child entries when value is a YAML mapping (empty otherwise)
}

type GitHubOrg {
  name: String! # Organization name (e.g. "elastic")
  teams_loaded: Boolean! # Whether teams were fetched from the API
  team_count: Int! # Number of teams (0 if not loaded)
  team: [GitHubTeam!]! # All teams in the organization
}

type GitHubTeam {
  slug: String! # URL slug (e.g. "clients-team")
  name: String! # Display name
  description: String # Team description (nullable)
}

type Span {
  filename: String! # Relative file path from repo root
  begin_line: Int! # 1-based starting line number
  end_line: Int! # 1-based ending line number
}
```

---

## Span Convention (Error Locations)

The lint engine extracts file location info from query output fields named `filename`, `begin_line`, and `end_line`. To
provide these, traverse one of the span edges using an **alias** that starts with `span_:`:

**File-level span** (points to the whole SKILL.md):

```graphql
span_: span {
    filename @output
    begin_line @output
    end_line @output
}
```

**Frontmatter-level span** (points to the frontmatter block; use `@optional` because frontmatter may not exist):

```graphql
span_: frontmatter_span @optional {
    filename @output
    begin_line @output
    end_line @output
}
```

**Field-specific spans** (point to the exact line of a frontmatter field; prefer these over `frontmatter_span` for field-level lints):

```graphql
span_: name_span @optional {
    filename @output
    begin_line @output
    end_line @output
}
```

Available field spans on `Skill`: `name_span`, `description_span`, `compatibility_span`. Use `frontmatter_end_span` for lints about missing fields (points to the closing `---` line).

**Reference-level span** (points to the exact line in the source file where a path reference appears; use inside a `referenced_path` block):

```graphql
referenced_path {
    # ... filters and outputs ...
    span_: span {
        filename @output
        begin_line @output
        end_line @output
    }
}
```

The `span_:` prefix is a Trustfall edge alias. It prevents the edge name from colliding with output field names. If omitted, the finding will still fire but without file/line information.

---

## Error Template Syntax

The `per_result_error_template` field uses Handlebars syntax. Placeholders are `{{name}}` where `name` is either:

1. An `@output` field name from the query (or its alias if `@output(name: "alias")` was used).
2. A key from the `arguments` map.

Example:

```ron
per_result_error_template: Some("{{skill_file_path}} description is {{description_word_count}} words (minimum {{min_words}})"),
```

Here `skill_file_path` and `description_word_count` come from `@output` fields, and `min_words` comes from the
`arguments` map.

---

## Arguments Map

The `arguments` field is a map of `String -> Value`. Values use RON literal syntax:

| Type    | RON Syntax       | Example     |
| ------- | ---------------- | ----------- |
| String  | `"text"`         | `"scripts"` |
| Integer | bare number      | `20`        |
| Boolean | `true` / `false` | `true`      |
| Float   | `1.0`            | `3.14`      |

Reference arguments in queries with the `$` prefix: if `arguments` has key `"max_lines"`, write
`@filter(op: ">", value: ["$max_lines"])`.

An empty arguments map is written as `arguments: {},`.

---

## Common Patterns

### Pattern 1: Check that a required field is missing

Finds skills where `name` is null (missing from frontmatter).

```ron
SkillLint(
    id: "skill_missing_name",
    human_readable_name: "SKILL.md frontmatter missing name",
    description: "A SKILL.md file's frontmatter is missing the required `name` field.",
    lint_level: Deny,
    reference_link: None,
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

Key points:

- First filter `has_frontmatter = true` so you only check skills that have frontmatter at all.
- `@filter(op: "is_null")` takes no `value` parameter.
- Use `frontmatter_span @optional` for the span because the lint is about a frontmatter field.

### Pattern 2: Regex validation on a field

Finds skill names that are not valid kebab-case.

```ron
SkillLint(
    id: "skill_name_invalid_format",
    human_readable_name: "skill name is not valid kebab-case",
    description: "The frontmatter `name` must be lowercase kebab-case.",
    lint_level: Deny,
    reference_link: None,
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
    error_message: "The `name` field must be valid kebab-case.",
    per_result_error_template: Some("{{skill_file_path}} has invalid name '{{name}}'"),
)
```

Key points:

- Use `not_regex` to match values that do NOT conform to a valid pattern (violation = not matching the good pattern).
- Use `regex` to match values that DO match a bad pattern (violation = matching the bad pattern).
- Always filter `is_not_null` before applying regex filters on nullable fields.

### Pattern 3: Numeric threshold

Finds descriptions that are too short (below a word count minimum).

```ron
SkillLint(
    id: "skill_description_too_short",
    human_readable_name: "skill description is too short",
    description: "The `description` field should be at least 20 words.",
    lint_level: Warn,
    reference_link: None,
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
    error_message: "The `description` field is too short. It should be at least 20 words.",
    per_result_error_template: Some("{{skill_file_path}} description is only {{description_word_count}} words (minimum {{min_words}})"),
)
```

Key points:

- The threshold value (`min_words`) is in `arguments`, making it available in the template too.
- Use `<` for "below minimum" checks, `>` for "above maximum" checks.

### Pattern 4: Check for a missing subdirectory (fold + count)

Finds skills that have no `scripts/` subdirectory.

```ron
SkillLint(
    id: "skill_has_no_scripts",
    human_readable_name: "skill has no scripts/ directory",
    description: "The skill does not have a scripts/ subdirectory.",
    lint_level: Allow,
    reference_link: None,
    query: r#"
    {
        Skill {
            sub_dir @fold
                    @transform(op: "count")
                    @filter(op: "=", value: ["$zero"]) {
                name @filter(op: "=", value: ["$scripts_dir_name"])
            }

            skill_file_path @output
            folder_name @output

            span_: span {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {
        "zero": 0,
        "scripts_dir_name": "scripts",
    },
    error_message: "This skill does not have a scripts/ subdirectory.",
    per_result_error_template: Some("{{skill_file_path}} has no scripts/ directory"),
)
```

Key points:

- `@fold` collapses all `sub_dir` edges into a single aggregate.
- Inside the fold, `name @filter(...)` narrows which subdirs are counted.
- `@transform(op: "count")` counts matching items.
- `@filter(op: "=", value: ["$zero"])` keeps only skills where the count is zero.
- The `"zero": 0` argument provides the integer value.

### Pattern 5: Check for a missing markdown section (fold on sections)

Finds skills missing a `## Guidelines` section.

```ron
SkillLint(
    id: "skill_missing_guidelines_section",
    human_readable_name: "SKILL.md missing Guidelines section",
    description: "The SKILL.md body should contain a '## Guidelines' section.",
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
                heading @filter(op: "regex", value: ["$guidelines_pattern"])
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
        "guidelines_pattern": "(?i)^guidelines?$",
    },
    error_message: "SKILL.md is missing a 'Guidelines' section.",
    per_result_error_template: Some("{{skill_file_path}} has no '## Guidelines' section"),
)
```

Key points:

- Same fold/count/filter pattern as Pattern 4, applied to the `section` edge.
- `heading @filter(op: "regex", ...)` matches the section heading text.
- `(?i)` makes the regex case-insensitive.
- `body_line_count > 0` avoids flagging empty files.

### Pattern 6: Cross-skill duplicate detection using `all_other_skills`

Finds skills that share the same `name` value.

```ron
SkillLint(
    id: "skill_duplicate_name",
    human_readable_name: "duplicate skill name",
    description: "Two or more skills share the same `name` value in their frontmatter.",
    lint_level: Deny,
    reference_link: None,
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
    error_message: "Multiple skills share the same `name` value.",
    per_result_error_template: Some("skill '{{this_path}}' has duplicate name '{{name}}' — also used by {{duplicate_path}}"),
)
```

Key points:

- `@tag(name: "my_name")` captures the current skill's name.
- `%my_name` in the nested `all_other_skills` block references that captured value.
- `@output(name: "this_path")` and `@output(name: "duplicate_path")` use aliases to disambiguate two `skill_file_path`
  outputs.
- `arguments` is empty (`{}`); the query uses only tags, not external arguments.

### Pattern 7: Structural checks using `DiscoveredSkillFile`

Finds SKILL.md files at the wrong nesting depth.

```ron
SkillLint(
    id: "skill_flat_layout",
    human_readable_name: "SKILL.md at wrong nesting depth",
    description: "A SKILL.md was found too close to the skills root.",
    lint_level: Deny,
    reference_link: None,
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
    error_message: "A SKILL.md file was found too close to the skills root.",
    per_result_error_template: Some("{{path}} is at depth {{depth}}, minimum is {{min_depth}}"),
)
```

Key points:

- Uses `DiscoveredSkillFile` instead of `Skill` because this checks files that might not be valid skills.
- `DiscoveredSkillFile` sees every SKILL.md regardless of depth; `Skill` only sees valid ones.

### Pattern 8: Cross-referencing folder names with tags

Checks that a skill name starts with its group folder name.

```ron
SkillLint(
    id: "skill_name_missing_group_prefix",
    human_readable_name: "skill name missing group folder prefix",
    description: "The frontmatter `name` should start with the group folder name.",
    lint_level: Deny,
    reference_link: None,
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

Key points:

- `@tag(name: "group")` on `group_folder` captures the group folder name.
- `@filter(op: "not_has_prefix", value: ["%group"])` checks the `name` field against that captured value.
- Tags let you compare two properties of the same object without hardcoding values.

### Pattern 9: Checking subdirectory file properties

Checks that a skill's `scripts/` directory doesn't mix programming languages.

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

            sub_dir {
                name @filter(op: "=", value: ["$scripts_dir_name"])
                unique_extension_count @filter(op: ">", value: ["$one"])
                unique_extensions @output(name: "script_extensions")
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
        "one": 1,
    },
    error_message: "A skill's scripts/ directory contains scripts in multiple languages.",
    per_result_error_template: Some("{{scripts_path}} contains multiple script languages: {{script_extensions}}"),
)
```

Key points:

- Traversing `sub_dir { ... }` navigates into subdirectories. If no subdir matches the inner filters, the parent Skill
  is excluded from results.
- `unique_extension_count` and `unique_extensions` are properties of `SubDir`.
- Output aliases (`@output(name: "scripts_path")`) prevent name collisions with parent fields.

### Pattern 10: Querying nested frontmatter metadata

Finds skills missing a `version` key inside the `metadata:` YAML mapping.

YAML like this uses a nested mapping:

```yaml
metadata:
  version: 0.1.0
  author: elastic
```

The `MetadataEntry` type has a `children` edge that exposes nested mapping entries. Use it by first matching the parent
key (`metadata`), then folding over its `children` to check for the nested key.

```ron
SkillLint(
    id: "skill_missing_version",
    human_readable_name: "SKILL.md frontmatter missing version in metadata",
    description: "Every skill must declare a `version` field inside its `metadata:` mapping.",
    lint_level: Deny,
    reference_link: None,
    query: r#"
    {
        Skill {
            has_frontmatter @filter(op: "=", value: ["$true"])

            metadata {
                key @filter(op: "=", value: ["$metadata_key"])

                children @fold
                         @transform(op: "count")
                         @filter(op: "=", value: ["$zero"]) {
                    key @filter(op: "=", value: ["$version_key"])
                }
            }

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
        "zero": 0,
        "metadata_key": "metadata",
        "version_key": "version",
    },
    error_message: "SKILL.md frontmatter is missing `version` inside the `metadata:` mapping.",
    per_result_error_template: Some("{{skill_file_path}} metadata is missing required field: version"),
)
```

Key points:

- `metadata { key @filter(...) }` navigates into the top-level metadata entries and selects the one with
  `key == "metadata"`.
- `children @fold @transform(op: "count") @filter(...)` counts child entries of that mapping whose key matches
  `"version"`. A count of zero means the field is missing.
- `children` is empty for scalar metadata entries, so this pattern only fires when the parent key is a YAML mapping that
  lacks the expected nested key.
- The same approach works for any nested key (e.g. `author`, `visibility`) by changing the `$version_key` argument.

### Pattern 11: Validating a metadata value against GitHub org teams

Finds skills whose `metadata.author` is not a valid team slug in the configured GitHub organization. The `teams_loaded`
guard ensures the lint produces no results when no GitHub token is available.

```ron
SkillLint(
    id: "skill_author_valid_github_team",
    human_readable_name: "metadata.author must be a valid GitHub team",
    description: "The `metadata.author` field must reference a valid team slug in the configured GitHub organization.",
    lint_level: Deny,
    reference_link: None,
    query: r#"
    {
        Skill {
            has_frontmatter @filter(op: "=", value: ["$true"])

            github_org {
                teams_loaded @filter(op: "=", value: ["$true"])

                team @fold
                    @transform(op: "count")
                    @filter(op: "=", value: ["$zero"]) {
                    slug @filter(op: "=", value: ["%author_value"])
                }
            }

            metadata {
                key @filter(op: "=", value: ["$metadata_key"])
                children {
                    key @filter(op: "=", value: ["$author_key"])
                    value @filter(op: "is_not_null")
                          @tag(name: "author_value")
                          @output(name: "author")
                }
            }

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
        "zero": 0,
        "metadata_key": "metadata",
        "author_key": "author",
    },
    error_message: "The metadata.author field must be a valid GitHub team slug.",
    per_result_error_template: Some("{{skill_file_path}} has invalid metadata.author '{{author}}' — not a recognized team in the GitHub organization"),
)
```

Key points:

- `github_org { teams_loaded @filter(...) }` ensures the lint only fires when GitHub teams were successfully loaded. If
  no token is available, `teams_loaded` is `false`, the filter excludes the `GitHubOrg` vertex, and the parent `Skill`
  produces no result row.
- `team @fold @transform(op: "count") @filter(op: "=", value: ["$zero"])` counts teams whose slug matches the author
  value. A count of zero means no matching team was found (i.e., the author is invalid).
- `%author_value` cross-references the metadata value captured with `@tag` in the parallel `metadata` traversal.
- The `github_org` edge is available on every `Skill` vertex and returns the same shared `GitHubOrg` singleton.

### Pattern 12: Detecting cross-skill path references

Finds skills that reference paths outside their own directory tree -- either via markdown links in SKILL.md or via
imports in script files. The `resolved_path` property normalizes relative paths to repo-root-relative, making it easy
to check whether a reference stays within the skill's own folder.

```ron
SkillLint(
    id: "skill_no_cross_references",
    human_readable_name: "skill references paths outside its own directory",
    description: "A skill should be self-contained. All local path references (markdown links, JS imports, etc.) must point to files within the skill's own directory.",
    lint_level: Deny,
    reference_link: None,
    query: r#"
    {
        Skill {
            path @tag(name: "skill_path") @output(name: "skill_path")
            skill_file_path @output

            referenced_path {
                resolved_path @filter(op: "is_not_null")
                              @filter(op: "not_has_prefix", value: ["%skill_path"])
                              @output(name: "target_path")
                raw_path @output(name: "raw_ref")
                kind @output(name: "ref_kind")

                span_: span {
                    filename @output
                    begin_line @output
                    end_line @output
                }
            }
        }
    }"#,
    arguments: {},
    error_message: "A skill references a path outside its own directory.",
    per_result_error_template: Some("{{skill_file_path}}: {{ref_kind}} '{{raw_ref}}' resolves to '{{target_path}}' which is outside {{skill_path}}"),
)
```

To also check script files inside subdirectories:

```ron
SkillLint(
    id: "skill_script_no_cross_references",
    human_readable_name: "script file references paths outside its skill directory",
    description: "Script files within a skill should not import or reference files outside the skill's own directory.",
    lint_level: Deny,
    reference_link: None,
    query: r#"
    {
        Skill {
            path @tag(name: "skill_path") @output(name: "skill_path")
            skill_file_path @output

            sub_dir {
                file {
                    name @output(name: "file_name")
                    path @output(name: "file_path")

                    referenced_path {
                        resolved_path @filter(op: "is_not_null")
                                      @filter(op: "not_has_prefix", value: ["%skill_path"])
                                      @output(name: "target_path")
                        raw_path @output(name: "raw_ref")
                        kind @output(name: "ref_kind")

                        span_: span {
                            filename @output
                            begin_line @output
                            end_line @output
                        }
                    }
                }
            }
        }
    }"#,
    arguments: {},
    error_message: "A script file references a path outside its skill's directory.",
    per_result_error_template: Some("{{file_path}}: {{ref_kind}} '{{raw_ref}}' resolves to '{{target_path}}' which is outside {{skill_path}}"),
)
```

Key points:

- `Skill.referenced_path` exposes links and images extracted from SKILL.md. `SubDirFile.referenced_path` exposes
  imports and source commands extracted from script files (JS/TS, Python, Shell).
- `resolved_path` is the raw relative path normalized to a repo-root-relative path. It is null when the path cannot be
  resolved (e.g. if it would escape the repo root). Filter with `is_not_null` before comparing.
- `@tag(name: "skill_path")` captures the skill's directory path. `not_has_prefix` then checks whether the resolved
  reference target starts with that prefix. If it does not, the reference points outside the skill.
- Place `span_: span { ... }` **inside** the `referenced_path` block (not on the outer `Skill`) so the error preview
  points to the exact import/link line in the source file rather than line 1 of SKILL.md.
- `SubDirFile.content` (nullable `String`) provides the raw text content of subdirectory files. It is null for binary
  files or files exceeding 1 MB. This can be used with regex filters for ad-hoc content checks beyond structured
  reference extraction.
- Supported `kind` values: `markdown_link`, `markdown_image`, `js_import`, `js_require`, `js_dynamic_import`,
  `python_relative_import`, `shell_source`.

### Pattern 13: Detecting directories without a SKILL.md using `DiscoveredDirectory`

Finds directories at the expected skill depth that contain files but have no SKILL.md. This catches directories that
look like they should be skills but are missing their definition file.

```ron
SkillLint(
    id: "directory_missing_skill_file",
    human_readable_name: "directory at skill depth has no SKILL.md",
    description: "A directory at the expected skill nesting depth contains files but no SKILL.md.",
    lint_level: Warn,
    reference_link: None,
    query: r#"
    {
        DiscoveredDirectory {
            depth @filter(op: "=", value: ["$skill_depth"])
            has_skill_file @filter(op: "=", value: ["$false"])
            file_count @filter(op: ">", value: ["$zero"]) @output

            name @output
            path @output

            span_: span {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {
        "skill_depth": 2,
        "zero": 0,
        "false": false,
    },
    error_message: "A directory at the expected skill depth has files but no SKILL.md.",
    per_result_error_template: Some("{{path}} has {{file_count}} file(s) but no SKILL.md"),
)
```

Key points:

- Uses the `DiscoveredDirectory` entry point, which walks all directories at depth >= 2 regardless of SKILL.md presence.
- `has_skill_file @filter(op: "=", value: ["$false"])` selects only directories without a SKILL.md.
- `file_count > 0` avoids flagging empty directories.
- `depth` counts directory path components under the skills root. Skill directories are typically at depth 2 (`group/skill-name`).

### Pattern 14: Detecting misplaced files at the skill root using `root_file`

Finds skills that have `.js` or `.ts` files sitting directly in the skill folder instead of inside `scripts/`.

```ron
SkillLint(
    id: "skill_root_has_script_files",
    human_readable_name: "script files at skill root instead of scripts/",
    description: "Script files (.js, .ts, etc.) should be placed in the scripts/ subdirectory, not directly in the skill folder.",
    lint_level: Deny,
    reference_link: None,
    query: r#"
    {
        Skill {
            skill_file_path @output

            root_file {
                extension @filter(op: "regex", value: ["$script_ext_pattern"])
                name @output(name: "file_name")
                path @output(name: "file_path")
            }

            span_: span {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {
        "script_ext_pattern": "^(js|mjs|cjs|jsx|ts|mts|cts|tsx|py|sh|bash|zsh)$",
    },
    error_message: "Script files should be in scripts/, not at the skill root.",
    per_result_error_template: Some("{{file_path}} is a script file at the skill root — move it to scripts/"),
)
```

Key points:

- `root_file` is an edge on `Skill` that lists non-SKILL.md files sitting directly in the skill folder.
- The `root_file` edge returns `SubDirFile` vertices, so all the same properties are available (name, extension, path, is_data_file, content, referenced_path).
- The regex matches common script file extensions. Adjust the pattern to match your project's conventions.
- If no root files match the filter, the parent Skill is excluded from results (no false positives).

### GitHub Token Configuration

GitHub-dependent lint queries require a valid token. The validator resolves the token in this order:

1. `GITHUB_TOKEN` environment variable
2. `GH_TOKEN` environment variable (gh CLI convention)
3. Output of `gh auth token` (for local dev with gh CLI installed)

When no token is available, GitHub-dependent lints are silently skipped (the `teams_loaded` guard prevents false
positives).

In CI (GitHub Actions), `${{ secrets.GITHUB_TOKEN }}` is automatically available and has read access to organization
teams.

The GitHub organization name defaults to `"elastic"` and can be overridden in `.skill-validator.toml`:

```toml
github_org = "elastic"
```

---

## Blank Template

Copy this as a starting point for new lint rules:

```ron
SkillLint(
    id: "CHANGE_ME",
    human_readable_name: "CHANGE_ME",
    description: "CHANGE_ME",
    lint_level: Deny,
    reference_link: None,
    query: r#"
    {
        Skill {
            // Add your filters and outputs here

            skill_file_path @output

            span_: span {
                filename @output
                begin_line @output
                end_line @output
            }
        }
    }"#,
    arguments: {},
    error_message: "CHANGE_ME",
    per_result_error_template: Some("CHANGE_ME"),
)
```

---

## Checklist Before Submitting a New Lint

1. The `id` is unique, descriptive, and uses snake_case.
2. The `query` is wrapped in `r#"..."#`.
3. Every `$variable` in the query has a corresponding key in `arguments`.
4. Every `%tag` in the query has a corresponding `@tag(name: "...")` earlier in the query.
5. The query includes a `span_:` block outputting `filename`, `begin_line`, `end_line`.
6. The `per_result_error_template` only references `@output` fields and `arguments` keys.
7. Nullable fields (like `name`, `description`) are checked with `is_not_null` before applying other filters.
8. The `lint_level` is appropriate: `Deny` for hard requirements, `Warn` for recommendations, `Allow` for opt-in checks.
9. The file has a `.ron` extension.
10. The file is placed in a directory listed in `custom_lint_dirs` in `.skill-validator.toml`.
11. The lint table in `README.md` is updated to include this lint (ID, description, level, and any other required
    columns).
