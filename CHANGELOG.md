# Changelog

## 0.5.0 (2026-10-26)

Teams: list them, filter projects by team, and move projects between teams.

### Breaking changes

- **`Project.teams` is now `Vec<Team>`** (id + name), not `Vec<String>`.
  `GET /v2/projects` returns team objects, so listing any project on a team
  failed to parse before this.

### Additions

- `list teams` (`--search`, or `--project <locator>` for one project's teams).
- `list projects --team <id>` / `--no-team`.
- `add project <locators…> --team <id>` and `remove project <locators…> --team <id>`.
- `move project <locators…> --from <id> --to <id>`: adds to the target team,
  then removes from the source. If the add fails nothing is removed; if the
  remove fails the error says the projects are now on both teams. `$revision`
  suffixes are stripped. The API's destructive `replace` action is never sent.
- `add`, `remove` and `move` are also new MCP tools.

## 0.4.0 (2026-08-18)

The CLI and MCP server now share one set of operation declarations (`src/ops/`),
so the two surfaces expose identical operations with identical parameters.

### Breaking changes

- **MCP arg shapes changed.** Per-entity fields replace the generic
  `parent`/string `id`; list entities are plural (`{"entity": "issues", ...}`),
  matching the CLI. Agent clients pick the new shapes up from `tools/list`;
  saved call examples must be updated (see README → Migrating to the unified
  surface).
- **The standalone `snippet_match` MCP tool is gone.** Use `get` with
  `entity: "snippet_match"`.
- **`list snippet-locations` is paginated.** Both surfaces page over the
  underlying snippets (`--page`/`--count`) and return a page object instead of
  an unbounded array.
- **CLI:** `list dependencies` no longer accepts `--revision` as an
  alternative to the positional revision argument.
- `update project` with no fields to change is rejected instead of sending an
  empty update.

### Additions

- MCP gains `get snippet`, paged `list snippets`, `list snippet_paths`, and
  issue-category auto-probe; the CLI gains `update --url/--policy-id/
  --default-branch` and real pagination on `list revisions` and
  `list dependencies`.
- `page`/`count` values below 1 are clamped to 1 (`count` is capped at 100).
- MCP calls using the legacy arg shapes fail with a migration hint naming
  this change.
- **Issues can be ignored and unignored** — the first issue write surface,
  as new top-level verbs on both surfaces. CLI: `ignore issue <id>
  --category <c> [--notes <comment>] [--reason <r>]` and `unignore issue
  <id> --category <c>`; MCP: new `ignore` and `unignore` tools. Ignoring an
  already-ignored issue is refused with a prompt to unignore first (see ADR
  0002), and `--reason` is accepted on vulnerability issues only (FOSSA
  never displays reasons elsewhere). Requires a full API token (push-only
  tokens cannot write issues).
