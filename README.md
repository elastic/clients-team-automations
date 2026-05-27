<img align="right" width="auto" height="auto" src="https://www.elastic.co/static-res/images/elastic-logo-200.png"/>

# Elastic Clients Team Automations

Contains shared reusable GitHub Actions workflows of the clients team.

## Workflows

### AI Backport Resolver (`ai-backport-resolver.yml`)

Reusable `workflow_call` workflow. When the backport bot posts a failure comment on a PR, resolves cherry-pick conflicts using Claude (via LiteLLM) and opens a backport PR.

**Usage** — add to your repo as a thin caller:

```yaml
# .github/workflows/resolve-conflicts.yml
name: AI Backport Resolver
on:
  issue_comment:
    types: [created]
jobs:
  resolve:
    uses: elastic/clients-team-automations/.github/workflows/ai-backport-resolver.yml@main
    with:
      comment_body: ${{ github.event.comment.body }}
      comment_user: ${{ github.event.comment.user.login }}
      pr_number: ${{ github.event.issue.number }}
    secrets:
      LITELLM_API_KEY: ${{ secrets.LITELLM_API_KEY }}
```

**Requires**: `LITELLM_API_KEY` secret in the calling repo.

## Agent label convention

Workflows that auto-fix issues on demand use the label prefix `agent: <task>`.

Adding an `agent: <task>` label to an issue signals that an AI agent should process it. Each repo registers which label it handles:

| Label | Repo | What it does |
|---|---|---|
| `agent: kibana type check` | `elastic/elasticsearch-specification` | Reads the issue, locates the relevant spec types, and opens a fix PR |

To add a new agent: create a workflow in your repo that triggers on `issues: labeled`, checks for your label, and implements the fix.

## License

Elastic Clients Team Automations is licensed under the MIT license.
