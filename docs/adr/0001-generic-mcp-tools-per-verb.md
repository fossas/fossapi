# Generic MCP tools per verb, not per operation

The MCP server exposes exactly three tools — `get`, `list`, `update` — each
taking an `entity` discriminator, rather than one tool per operation
(`get_issue`, `list_snippets`, …). The CLI and the MCP server are one
presentation layer and must stay consistent, so the tools mirror the CLI's
verb-first grammar; a small tool list also keeps agent contexts lean.

## Considered Options

One tool per operation was rejected: its main draws are per-tool permissioning
and discoverability, but authorization is enforced by the FOSSA app behind the
API token (the tool surface adds nothing), and entity discoverability is
handled by the tool descriptions and self-describing input schemas.

## Consequences

Adding an operation never changes the MCP tool list — clients' saved tool
configurations stay valid as the operation set grows. The cost is that the
`entity` tag becomes load-bearing wire format: renaming an entity is a
breaking change to every saved call, not just a CLI rename.
