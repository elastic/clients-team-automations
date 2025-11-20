# Gencheck - Generated Files Check Action

A GitHub Action that detects edits to generated files in pull requests, with configurable patterns and customizable commenting.

See [gencheck.yml](../.github/workflows/gencheck.yml) for an example usage.

## Usage

### Basic Example

```yaml
name: 'Check Generated Files'
on:
  pull_request_target:
    types: [opened, synchronize, labeled, unlabeled]

permissions:
  contents: read
  pull-requests: write

jobs:
  check-generated:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: 'Check for Generated Files'
        uses: ./gencheck
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          file_patterns: '**/*.go,**/*.pb.go'
          marker_patterns: 'Code generated.*DO NOT EDIT'
```

### Advanced Example with Custom Template

```yaml
- name: 'Check for Generated Files'
  uses: ./gencheck
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    file_patterns: '**/*.go,**/*.ts'
    marker_patterns: 'Code generated.*DO NOT EDIT,@generated'
    skip_label: 'allow-generated-edits'
    post_comment: 'true'
    fail_on_detected: 'true'
    comment_template: |
      ## 🤖 Generated Files Alert
      
      This PR modifies **{{file_count}}** generated file(s):
      
      {{file_list}}
      
      ### What should I do?
      
      Generated files are auto-generated and manual edits may be lost.
      
      **Options:**
      - ✅ Add the `{{skip_label}}` label if these changes are intentional
      - 💬 [Open an issue](https://github.com/{{owner}}/{{repo}}/issues/new) if you need help
      
      _This check can be bypassed by adding the skip label to PR #{{pr_number}}_
```

## Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `github_token` | GitHub token for API access (requires `contents: read, pull-requests: write`) | Yes | - |
| `file_patterns` | Comma-separated glob patterns for files to check | No | `**/*.go` |
| `marker_patterns` | Comma-separated regex patterns to detect generated file markers | No | `Code generated.*DO NOT EDIT,Code generated from.*specification` |
| `skip_label` | Label name that allows skipping this check | No | `skip: gencheck` |
| `post_comment` | Whether to post a comment on the PR when generated files are detected | No | `true` |
| `fail_on_detected` | Whether to fail the check when generated files are detected (without skip label) | No | `true` |
| `comment_template` | Custom comment template with variables (see below) | No | _(uses default template)_ |

## Template Variables

When using `comment_template`, you can use the following template variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `{{file_count}}` | Number of generated files detected | `3` |
| `{{file_list}}` | Formatted markdown list of files | `- \`path/to/file.go\`<br>- \`path/to/other.go\`` |
| `{{skip_label}}` | The skip label name | `skip: gencheck` |
| `{{pr_number}}` | Pull request number | `42` |
| `{{repo}}` | Repository name | `my-repo` |
| `{{owner}}` | Repository owner | `my-org` |

### Template Examples

#### Minimal Template

```yaml
comment_template: |
  ⚠️ This PR modifies {{file_count}} generated file(s).
  Add the `{{skip_label}}` label to bypass.
```

#### Detailed Template with Custom Formatting

```yaml
comment_template: |
  ## 🔔 Generated Files Detected in PR #{{pr_number}}
  
  **Repository:** {{owner}}/{{repo}}  
  **Files affected:** {{file_count}}
  
  ### Modified Files
  {{file_list}}
  
  ### 🚨 Important
  These are generated files! Manual edits will likely be lost.
  
  #### Options:
  - Add the `{{skip_label}}` label if this is intentional
  - [Open an issue](https://github.com/{{owner}}/{{repo}}/issues/new) to discuss
  - Revert and update source templates instead
```

#### Team-Specific Template

```yaml
comment_template: |
  ## ⚠️ Generated Code Modified
  
  @{{owner}}/backend-team - This PR touches {{file_count}} generated file(s):
  
  {{file_list}}
  
  These files are managed by our code generation pipeline. 
  Please review the [Code Generation Guide](https://wiki.example.com/codegen) 
  before proceeding.
  
  **Options:**
  - Add `{{skip_label}}` label if intentional
  - [Open an issue](https://github.com/{{owner}}/{{repo}}/issues/new) for guidance
```

## Outputs

| Output | Description | Example |
|--------|-------------|---------|
| `detected` | Boolean string indicating if generated files were found | `true` or `false` |
| `files` | JSON array of detected generated files | `["path/to/file.go", "other.go"]` |
| `skipped` | Boolean string indicating if check was skipped due to label | `true` or `false` |

## Permissions

This action requires the following permissions:

```yaml
permissions:
  contents: read        # To read file contents
  pull-requests: write  # To post/update comments
```

## License

See repository LICENSE file.
