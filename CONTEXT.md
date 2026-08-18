# fossapi

A CLI and MCP server exposing the FOSSA API to humans and agents through one
shared set of operation declarations.

## Language

**Operation**:
One verb applied to one entity (e.g. get issue, list snippets). Declared once
and exposed identically by both surfaces.
_Avoid_: endpoint, command

**Verb**:
One of `get`, `list`, `update`, `ignore`, `unignore` — the top-level grouping
of operations. Each verb is one shared enum and one MCP tool. (In code the
enums are spelled `GetCommand`/`ListCommand`/etc.; "command" in a type name
means verb, not operation.)
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

### Issues

**Issue**:
A detected problem in a dependency — a vulnerability, licensing conflict, or
quality concern. Always scoped to exactly one Category and identified by a
numeric ID, not a locator.
_Avoid_: alert, finding

**Category**:
Which of the three issue kinds an issue belongs to: vulnerability, licensing,
or quality. Every issue read and write requires one.
_Avoid_: type, kind

**Ignore**:
The only issue status transition: an active issue becomes ignored, optionally
carrying Notes and a Reason. There is no "resolved" status — when people say
"resolve an issue" they mean ignore it.
_Avoid_: resolve, suppress, dismiss, mute

**Unignore**:
Reverting an Ignore, returning the issue to active.
_Avoid_: reopen, reactivate

**Notes**:
Free text attached to an Ignore explaining it (e.g. "false positive patch").
Not a Comment.
_Avoid_: comment, message

**Reason**:
One of a closed set of structured explanations attached to a vulnerability
Ignore (Fixed, Vulnerable code not in execute path, Other, …). Vulnerability
issues only — nothing in FOSSA ever displays a reason for licensing or
quality ignores.
_Avoid_: justification, cause

**Issue exception**:
An org- or policy-wide ignore that can expire, distinct from ignoring one
issue. Exists in FOSSA but is not modeled in fossapi.

### Adjacent FOSSA concepts (not issue ignores)

**Comment**:
A separate FOSSA feature: discussion threads attached to a package, org-wide
across versions. Unrelated to an Ignore's Notes. Not modeled in fossapi.

**Package ignore**:
A separate FOSSA feature: hiding a dependency from the inventory entirely
("Ignore package" in the UI). Not an issue status change. Not modeled in
fossapi.
_Avoid_: conflating with Ignore
