# Issue writes add a client-side state guard the server doesn't have

FOSSA's `PUT /v2/issues/` is an unguarded upsert when targeting by ID: it
silently ignores the `status` query filter, and re-ignoring an already-ignored
issue overwrites its notes/reason and resets its ignored-at timestamp (the
server's `ON CONFLICT DO UPDATE` on `IssueResolutions`). We decided fossapi's
`ignore issue` pre-fetches the issue and **refuses** an action whose target
state already fully holds ("already ignored; unignore it first"), mirroring
the web UI, which only ever offers Ignore on active issues and Unignore on
ignored ones. Partially ignored issues (org-wide rollup: ignored in some
projects, active in others) accept both actions, like the UI's global issue
view.

## Considered Options

Mirroring the server (letting re-ignore silently overwrite) would have given
one-step "edit the notes" — the raw API is in fact the only way to do that in
one step — but an agent retrying or mis-aiming an ignore would clobber a
human's hand-written justification without any signal. We chose the guard and
made editing notes a deliberate two-step: unignore, then re-ignore with the
new notes.

## Consequences

- Every issue write costs a pre-flight GET (and a post-write refresh GET);
  there is a benign race window between fetch and write.
- `count: 0` from the server, after the pre-flight has ruled out a wrong ID or
  category, means only "not visible to this token".
- The same UI-mirroring stance also rejects a `reason` on non-vulnerability
  ignores: the server stores one for any category, but only vulnerability
  ignores ever display it (UI, SBOM/VEX) — elsewhere it is write-only.
