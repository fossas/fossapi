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
| `list` | List projects, revisions, dependencies, or issues |
| `update` | Update project metadata (title, description, url, public) |

## Locators

FOSSA uses locators to identify entities:

- **Project**: `custom+{org_id}/{project_name}`
- **Revision**: `custom+{org_id}/{project_name}${revision_ref}`
- **Dependency**: `{fetcher}+{package}${version}` (e.g., `npm+lodash$4.17.21`)
