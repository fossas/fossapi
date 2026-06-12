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

```bash
# List vulnerabilities
fossapi list issues --category vulnerability

# List licensing issues
fossapi list issues --category licensing

# Get a specific issue
fossapi get issue 12345
```

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

# Flat report: every match location (first-party file -> matched package)
fossapi list snippet-locations "custom+1/my-project\$abc123"

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

| Tool | Description |
|------|-------------|
| `get` | Fetch a single project, revision, or issue by ID |
| `list` | List projects, revisions, dependencies, issues, or snippet match locations |
| `update` | Update project metadata (title, description, url, public) |
| `snippet_match` | Drill into one snippet match: the matched first-party and reference code |

> **Snippets over MCP:** use `list` with `entity: snippet` and `parent: <revision
> locator>` (optional `path` and `with_lines`) to map third-party matches to
> first-party files, then `snippet_match` to drill into a single match. Snippets
> don't support `get` or `update`.

## Locators

FOSSA uses locators to identify entities:

- **Project**: `custom+{org_id}/{project_name}`
- **Revision**: `custom+{org_id}/{project_name}${revision_ref}`
- **Dependency**: `{fetcher}+{package}${version}` (e.g., `npm+lodash$4.17.21`)
- **Snippet**: identified by its parent revision locator plus a snippet ID (a string)
