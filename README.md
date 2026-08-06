<img align="right" width="auto" height="auto" src="https://www.elastic.co/static-res/images/elastic-logo-200.png"/>

# Elastic Clients Team Automations

Shared reusable GitHub Actions workflows for Elastic client repositories.

---

## Workflows

### 1. `auto-pr.yml` — AI-driven issue fix

Reads a GitHub issue, finds relevant files in your repo, calls Claude (via LiteLLM) to produce a fix, and opens a PR. Triggered by adding an `auto-pr` label (or any label starting with `auto-pr:`) to an issue.

#### Setup

**Step 1** — Add a thin caller workflow to your repo:

```yaml
# .github/workflows/auto-pr.yml
name: Auto PR
on:
  issues:
    types: [labeled]

jobs:
  auto-pr:
    if: startsWith(github.event.label.name, 'auto-pr')
    uses: elastic/clients-team-automations/.github/workflows/auto-pr.yml@main
    with:
      issue_number: ${{ github.event.issue.number }}
      search_directory: src   # optional — directory to search for relevant files
    secrets:
      LITELLM_API_KEY: ${{ secrets.LITELLM_API_KEY }}
```

**Step 2** — Add a context file at `.github/auto-pr-context.md` in your repo:

```markdown
# Auto PR context — my-repo

Brief description of what this repo contains and what kinds of issues are expected.

## File layout
- `src/` — main source files
- `types/` — type definitions

## Fix conventions
- Describe what kind of changes Claude should make
- e.g. "Only modify files under `src/types/`. Do not touch generated files."

## Search hints
- Mention what identifiers or patterns in the issue body map to which directories
```

**Step 3** — Add `LITELLM_API_KEY` as a repository secret (Settings → Secrets → Actions).

**Step 4** — Create the `auto-pr` label in your repo (Settings → Labels), or use `auto-pr: <context>` for a more descriptive variant.

#### How it works

1. An issue is labeled with `auto-pr` (or `auto-pr: <description>`)
2. The workflow reads the issue body and `.github/auto-pr-context.md`
3. It extracts identifiers (type names, symbols) from the issue and searches `search_directory` for relevant files
4. Claude receives the issue + file contents and returns a JSON patch with the file changes
5. The changes are applied, committed, and a PR is opened — with a comment posted on the original issue

#### Tips for a good context file

The quality of the fix depends heavily on `.github/auto-pr-context.md`. Include:
- What the repo does and what its conventions are
- Which directory to look in for fixes
- Common fix patterns with examples

See [elasticsearch-specification's context file](https://github.com/elastic/elasticsearch-specification/blob/main/.github/auto-pr-context.md) as a reference.

---

### 2. `ai-backport-resolver.yml` — AI backport conflict resolver

When the backport bot posts a failure comment on a PR, resolves cherry-pick conflicts using Claude and opens a backport PR automatically.

#### Setup

**Step 1** — Add a thin caller workflow to your repo:

```yaml
# .github/workflows/resolve-conflicts.yml
name: AI Backport Resolver
on:
  issue_comment:
    types: [created]

jobs:
  resolve:
    if: |
      (github.event.comment.user.login == 'github-actions[bot]' ||
       github.event.comment.user.login == 'elastic-vault-github-plugin-prod[bot]') &&
      contains(github.event.comment.body, 'To backport manually, run these commands')
    uses: elastic/clients-team-automations/.github/workflows/ai-backport-resolver.yml@main
    with:
      comment_body: ${{ github.event.comment.body }}
      comment_user: ${{ github.event.comment.user.login }}
      pr_number: ${{ github.event.issue.number }}
    secrets:
      OPENROUTER_API_KEY: ${{ secrets.OPENROUTER_API_KEY }}
      OPENROUTER_BASE_URL: ${{ secrets.OPENROUTER_BASE_URL }}
```

The job-level `if` filters out unrelated comments before the reusable workflow is even invoked, so the repo's Actions tab isn't full of runs for every comment.

**Step 2** — Add `OPENROUTER_API_KEY` and `OPENROUTER_BASE_URL` as repository secrets.

#### How it works

1. The backport bot posts a failure comment starting with `"The backport to..."` on a merged PR
2. The workflow parses the comment to extract the target branch and commit SHA
3. It attempts the cherry-pick — if no conflicts, a backport PR is created directly
4. If there are conflicts, Claude resolves them file by file
5. A backport PR is opened and a comment is posted on the original PR

---

## `auto-pr` label convention

The `auto-pr` prefix is a shared convention across Elastic client repos. Adding an `auto-pr` label to an issue signals that an AI agent should process it and open a fix PR. Workflows check `startsWith(github.event.label.name, 'auto-pr')` so both `auto-pr` and `auto-pr: <context>` trigger — the suffix is optional human-readable context.

| Label | Repo | What it does |
|---|---|---|
| `auto-pr: kibana type check` | `elastic/elasticsearch-specification` | Fixes spec type errors found by the Kibana type check pipeline |

To register your repo's agent, add a row to the table above in a PR to this README.

## License

Elastic Clients Team Automations is licensed under the MIT license.
