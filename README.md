# fossapi

A CLI and MCP server for querying the FOSSA API.

## Installation

### macOS / Linux

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/fossas/fossapi/releases/latest/download/fossapi-installer.sh | sh
```

### Windows (PowerShell)

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/fossas/fossapi/releases/latest/download/fossapi-installer.ps1 | iex"
```

### From source

```bash
cargo install --git https://github.com/fossas/fossapi
```

## Setup

Set your FOSSA API key:

```bash
export FOSSA_API_KEY=your_api_key_here
```

## CLI Usage

### Projects

```bash
# List all projects
fossapi list projects

# Get a specific project
fossapi get project "custom+1/my-project"

# Update project metadata
fossapi update project "custom+1/my-project" --title "New Title"
```

### Revisions

```bash
# List revisions for a project
fossapi list revisions "custom+1/my-project"

# Get a specific revision
fossapi get revision "custom+1/my-project\$abc123"
```

### Dependencies

```bash
# List dependencies for a revision
fossapi list dependencies "custom+1/my-project\$abc123"
```

### Issues

Issues come in three categories: `vulnerability`, `licensing`, and `quality`.
The API scopes every issue lookup to one category, so `--category` is required
when listing.

```bash
# List vulnerabilities
fossapi list issues --category vulnerability

# List licensing issues
fossapi list issues --category licensing

# Get a specific issue; searches each category in turn
fossapi get issue 12345

# Skip the search when you know the category
fossapi get issue 12345 --category licensing

# Ignore an issue with a comment
fossapi update issue 12345 --category licensing --ignore \
  --notes "false positive patch" --reason other

# Revert the ignore
fossapi update issue 12345 --category licensing --unignore
```

Ignoring supports an optional `--notes` free-text comment and a `--reason`
(one of `fixed`, `under-investigation`, `incorrect-data-found`,
`component-not-present`, `vulnerable-code-not-present`,
`vulnerable-code-not-in-execute-path`,
`vulnerable-code-cannot-be-controlled-by-adversary`,
`inline-mitigations-already-exist`, `other`). Issue writes require a full API
token; push-only tokens can only read.

Ignoring an issue that is already fully ignored fails with a prompt to
unignore it first — changing an existing ignore's notes is a deliberate
two-step (`--unignore`, then `--ignore --notes ...`), matching the web UI. An
issue ignored in some projects but active in others accepts both actions,
which then apply org-wide.

### Snippets

Snippet scanning finds third-party (open-source) code copied into your
first-party files. Each **snippet** is a matched OSS package; its **matches**
are the first-party files where that code was found. The snippet surface is
read-only and scoped to a single revision.

```bash
# List snippets (matched OSS packages) in a revision
fossapi list snippets "custom+1/my-project\$abc123"

# Restrict to a file/directory subtree (defaults to the repo root)
fossapi list snippets "custom+1/my-project\$abc123" --path /src

# Show the file/directory tree where snippets were detected
fossapi list snippet-paths "custom+1/my-project\$abc123"

# Flat report: every match location (first-party file -> matched package).
# Paginated over snippets: each page returns every location of --count snippets.
fossapi list snippet-locations "custom+1/my-project\$abc123" --page 1 --count 20

# ...and resolve the first-party line range for each match (extra API calls)
fossapi list snippet-locations "custom+1/my-project\$abc123" --with-lines

# Get a snippet's details, including its matched first-party files
fossapi get snippet "custom+1/my-project\$abc123" <snippet-id>

# Side-by-side match details (detected vs reference code) at a matched path
fossapi get snippet-match "custom+1/my-project\$abc123" <snippet-id> src/foo.rs
```

### Output Formats

```bash
# Pretty tables (default)
fossapi list projects

# JSON output
fossapi list projects --json
```

## MCP Server

Run as an MCP server for use with Claude Code or other AI tools:

```bash
fossapi mcp
```

### Configuration

Add to your MCP config:

```json
{
  "mcpServers": {
    "fossa": {
      "type": "stdio",
      "command": "fossapi",
      "args": ["mcp"],
      "env": {
        "FOSSA_API_KEY": "your_key"
      }
    }
  }
}
```

> **Note:** If `fossapi` isn't in your PATH, use the full path: `~/.cargo/bin/fossapi`

### Available Tools

The MCP tools mirror the CLI verbs exactly: each tool takes an `entity`
discriminator naming the subcommand, plus that subcommand's arguments (the
input schemas are generated from the same declarations the CLI parses into).

| Tool | Entities |
|------|----------|
| `get` | `project`, `revision`, `issue` (category optional — omitted probes all three), `snippet`, `snippet_match` |
| `list` | `projects`, `issues` (category required), `dependencies`, `revisions`, `snippets`, `snippet_locations`, `snippet_paths` |
| `update` | `project` (title, description, url, public, policy_id, default_branch), `issue` (ignore/unignore with optional notes and reason; category required) |

For example, `fossapi get issue 12345 --category licensing` is
`get {"entity": "issue", "id": 12345, "category": "licensing"}` over MCP, and
`fossapi update issue 12345 --category licensing --ignore --notes "false positive patch"`
is `update {"entity": "issue", "id": 12345, "category": "licensing", "ignore": true, "notes": "false positive patch"}`.

> **Snippets over MCP:** use `list` with `entity: snippet_locations` and
> `revision: <revision locator>` (optional `path` and `with_lines`) to map
> third-party matches to first-party files, then `get` with
> `entity: snippet_match` to drill into a single match.

Paged list operations take `page` and `count` (defaults 1 and 20; values are
clamped to at least 1 and `count` to at most 100). `snippet_locations` pages
over the underlying snippets, so one page can hold more or fewer rows than
`count`.

### Migrating to the unified surface

The CLI/MCP unification changed the MCP arg shapes (breaking for saved call
configs; live clients pick the new shapes up automatically from `tools/list`):

- Per-entity fields replace the old generic `parent`/string `id` — e.g.
  `get {"entity": "issue", "id": 12345}` (numeric id),
  `list {"entity": "revisions", "project": "custom+1/my-project"}`.
- List entities are plural (`projects`, `issues`, …), matching the CLI.
- The standalone `snippet_match` tool folded into
  `get {"entity": "snippet_match", ...}`.
- `list {"entity": "snippet_locations", ...}` is now paginated and returns a
  page object (`items`/`page`/`count`/`total`/`has_more`) instead of a bare
  array.

On the CLI, `list dependencies` now takes the revision positionally only
(`--revision` was removed). Calls using the old shapes fail with an error that
points back to this section.

## Locators

FOSSA uses locators to identify entities:

- **Project**: `custom+{org_id}/{project_name}`
- **Revision**: `custom+{org_id}/{project_name}${revision_ref}`
- **Dependency**: `{fetcher}+{package}${version}` (e.g., `npm+lodash$4.17.21`)
- **Snippet**: identified by its parent revision locator plus a snippet ID (a string)
