# fossapi - FOSSA API Rust Client

Rust library wrapping the FOSSA API with trait-based architecture.

## Entity Relationship Diagram

```mermaid
erDiagram
    Project ||--o{ Revision : "has many"
    Revision ||--o{ Dependency : "contains"
    Revision ||--o{ IssueScan : "triggers (future)"

    Project {
        String id "locator: custom+org/project"
        String title
        LatestRevision latest_revision
        ProjectIssues issues
    }

    Revision {
        String locator PK "custom+org/project$ref"
        bool resolved
        String source "cli or api"
        String source_type "cargo, npm, etc"
        u32 unresolved_issue_count
        DateTime created_at
    }

    Dependency {
        String locator PK "npm+package$version"
        u32 depth "1=direct, >1=transitive"
        Vec_LicenseInfo licenses
        Vec_DependencyIssue issues
    }

    IssueScan {
        u64 id PK
        String revision_locator FK
        String status
        DateTime completed_at
    }
```

## Architecture

```
Project (top-level container)
├── latest_revision: LatestRevision
│   └── locator → can fetch full Revision
└── revisions() → Vec<Revision>
    └── revision.dependencies() → Vec<Dependency>
```

## API Endpoints

| Entity | Endpoint | Notes |
|--------|----------|-------|
| Projects | `GET /v2/projects` | Paginated listing |
| Project | `GET /projects/{locator}` | Single project |
| Revisions | `GET /projects/{locator}/revisions` | Grouped by branch |
| Dependencies | `GET /v2/revisions/{locator}/dependencies` | For a revision |

## Traits

| Trait | Purpose | Example |
|-------|---------|---------|
| `Get` | Fetch single entity | `Project::get(&client, locator)` |
| `List` | Paginated listing | `Project::list_page(&client, query, page, count)` |
| `Update` | Modify entity | `Project::update(&client, locator, params)` |

## Models

- **Project** - Top-level container, implements Get/List/Update
- **Revision** - Snapshot at point in time, implements Get/List
- **Dependency** - Package dependency, implements List only (via revision)
- **LicenseInfo** - Can be simple string ("MIT") or full object

## Future Work

- **IssueScan** - Issue scans tied to revisions (not yet implemented)
