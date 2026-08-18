# Changelog

## 0.4.0 (unreleased)

The CLI and MCP server now share one set of operation declarations (`src/ops/`),
so the two surfaces expose identical operations with identical parameters.

### Breaking changes

- **MCP arg shapes changed.** Per-entity fields replace the generic
  `parent`/string `id`; list entities are plural (`{"entity": "issues", ...}`),
  matching the CLI. Agent clients pick the new shapes up from `tools/list`;
  saved call examples must be updated (see README → Migrating to 0.4).
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
- MCP calls using pre-0.4 arg shapes fail with a migration hint naming this
  change.
