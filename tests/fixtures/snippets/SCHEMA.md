# Snippet API — confirmed schema (live spike 2026-06-10)

Probed against `custom+58216/github.com/fossas/hack-house-docs$eda8d18...` (Alamofire snippets).
All endpoints `GET`, base `https://app.fossa.com/api`, bearer auth. Fixtures in this dir are real
responses (match-details trimmed to `-small`/`-partial` for test size).

## Deltas from the documented hypothesis (important)
- Snippet **`id` is a STRING** (`"1295019"`), not a number.
- List wrapper paginates with **`pageSize`** (not `count`): `{results, totalCount, page, pageSize}`.
- Extra Snippet fields present: `homeUrl`, `codeUrl`, `isVendored`, `isConverted`, `releaseDate?`.
- `licenses[]` items are `{signature, type}` (e.g. `type:"declared"`); `status`/`issueId` optional.
- `/snippets/paths` is **hierarchical**: `path=/` returns a directory with a `count`; you drill
  (`path=/Sources` → `/Sources/Networking` → files). `type` ∈ {`directory`,`file`}.
- Line numbers exist **only** in the match-details drill-in. Neither the list nor snippet-details
  carries per-match line ranges (details `matches[]` = `{path, matchPercentage, rejectionDetails?}`).
- For `kind:"file"` matches every line `isHighlighted` (whole-file match). Partial ranges only occur
  for `kind:"snippet"`.

## Shapes
- `GET …/snippets/paths?path=P` → `{paths:[{type,name,path,count}]}`
- `GET …/snippets?path=P&page&pageSize&sort` → `{results:[Snippet],totalCount,page,pageSize}`
  - Snippet (list): `{id,packageId,purl,locator,package,version,kind,highestMatchPercentage,
    homeUrl?,codeUrl?,releaseDate?,labels[],rejectionDetails?,licenses[],issueCounts,
    isVendored,isConverted,matchCount}`
  - `issueCounts`: `{licensing:{denied,flagged,unknown}, security:{critical,high,medium,low,unknown}}`
  - `rejectionDetails`: `{rejectedAt, rejectedBy}`
- `GET …/snippets/{id}` → `{snippet: Snippet + matches[] + otherVersions[]}`
  - `matches[]`: `{path, matchPercentage, rejectionDetails?}`
  - `otherVersions[]`: `{version, matchCount}`
- `GET …/snippets/{id}/matches/{urlencoded-path}` → `{matchDetails:{path,matchPercentage,
  referenceCode[],detectedCode[]}}`; each code line `{line, lineNumber, isHighlighted}`.
