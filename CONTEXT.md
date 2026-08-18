# fossapi

A CLI and MCP server exposing the FOSSA API to humans and agents through one
shared set of operation declarations.

## Language

**Operation**:
One verb applied to one entity (e.g. get issue, list snippets). Declared once
and exposed identically by both surfaces.
_Avoid_: endpoint, command

**Verb**:
One of `get`, `list`, `update` — the top-level grouping of operations. Each
verb is one shared enum and one MCP tool. (In code the enums are spelled
`GetCommand`/`ListCommand`/`UpdateCommand`; "command" in a type name means
verb, not operation.)
_Avoid_: action, method

**Entity**:
The thing a verb acts on (`project`, `issue`, `snippet_locations`, …). Appears
as the CLI subcommand name and as the `entity` discriminator in MCP arguments.
_Avoid_: resource, object

**Declaration**:
The single definition of an operation — its parameter struct and enum
variant — from which both surfaces derive their interface, documentation, and
schemas.

**Surface**:
A way of reaching the operations: the CLI (for humans) or the MCP server (for
agents). Surfaces are thin adapters; neither adds operations of its own.
_Avoid_: frontend, interface

**Parity**:
The guarantee that both surfaces expose exactly the same operations with the
same parameters.

**Pagination policy**:
The defaults and global bounds applied to `page`/`count` before an operation
runs. One policy for all operations; individual FOSSA endpoints may impose
their own tighter bounds.
