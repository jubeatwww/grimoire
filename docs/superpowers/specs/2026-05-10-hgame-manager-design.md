# HGame Manager Design

Date: 2026-05-10

## Purpose

Build a self-hosted personal management tool for an hgame archive stored on NAS.
The first version must make it easy to browse, search, classify, review, enrich,
download, and stage games without risking accidental writes to the existing file
library during development.

The current NAS library is mounted at `/mnt/games`. It currently uses legacy
top-level folders such as `ACT`, `ADV`, `ETC`, `LunaSoft`, `RPG`, `SIM+SLG`,
and `未分類`. These folders are treated as legacy locations, not as the final
taxonomy. The long-term goal is to organize the NAS into a consistent, browsable
format that also works well through a NAS file browser.

## Scope

Version 1 focuses on:

- Scanning `/mnt/games` as a read-only library source.
- Creating a searchable inventory of zip/rar files and any future game folders.
- Separating actual files from normalized game works.
- Browsing the library with a cover-first UI.
- Manually editing personal metadata.
- Searching DLsite for candidate metadata and importing confirmed results.
- Caching cover and preview images while preserving source URLs.
- Downloading existing `.zip` and `.rar` files to a local machine.
- Importing new files into staging through browser upload or a watch folder.
- Producing dry-run organization plans for future NAS cleanup.
- Configuring storage, metadata, and database sources through adapters.
- Exporting metadata to a portable JSON snapshot for backup or migration.

Version 1 does not include:

- Login or multi-user permissions.
- Public internet deployment support.
- Game launch or patch execution.
- Full download manager features such as queueing, pause, resume, or retry UI.
- Automatic destructive organization of the NAS.
- Automatic batch matching without user confirmation.

## Architecture

The selected architecture is a centralized server with shared web and desktop
clients.

- `hgame-server`: Rust API server running on the home server or k3s.
- Database: PostgreSQL as the primary metadata store.
- Optional database adapter: SQLite retained as a future local or fallback source.
- Library source: `/mnt/games` mounted read-only during development.
- Staging storage: writable app-managed storage for new imports.
- Asset cache: app-managed storage for downloaded cover and preview images.
- `hgame-ui`: React, TypeScript, and Vite SPA.
- `hgame-desktop`: Tauri v2 shell for Windows and macOS using the same SPA.
- Web version: the same SPA served through the server or a static web host.

The first version is a thin client model. The server owns NAS access, database
access, DLsite access, asset caching, scanning, staging, and download streaming.
The desktop app connects to the server API and provides local save-location
selection for downloads.

The server uses lightweight DDD-style boundaries without overbuilding:

- `domain`: core models and value types.
- `application`: scan, match, import, download, and organization workflows.
- `adapters/http`: Axum routes and API DTOs.
- `adapters/db`: PostgreSQL and future SQLite repositories.
- `adapters/library`: NFS read-only source and future writable NAS adapter.
- `adapters/staging`: upload and watch-folder storage.
- `adapters/metadata`: DLsite first, with room for other sources.
- `adapters/assets`: image download and cache management.
- `adapters/export`: portable metadata export and import.

## Configurable Sources and Adapters

Sources are configured explicitly instead of hard-coded into business logic.

Source types:

- Database source: PostgreSQL first, with SQLite kept as a later local or
  fallback adapter.
- Library source: read-only NFS source for `/mnt/games`.
- Staging source: upload storage and watch-folder storage.
- Writable library source: future NAS commit adapter, disabled during
  development.
- Metadata source: DLsite first, with the same interface available for future
  Steam, official-site, or custom sources.
- Asset source: local cache rooted in app-managed storage.

The application layer depends on source interfaces, not on concrete storage or
metadata implementations. This keeps PostgreSQL, SQLite, DLsite, NFS, staging,
and future sources replaceable without changing scan, match, import, download,
or organization rules.

Exports are produced from domain/application models, not by dumping one database
directly. The export format includes a schema version and can be imported later
for backup, migration, or recovery.

## Data Model

### GameWork

`GameWork` represents a normalized game work. It is not the same as a specific
zip or rar file.

Key fields:

- Canonical title.
- Original title.
- Display title.
- Circle, developer, and publisher.
- Official and source URLs.
- DLsite work id and source provenance.
- Description.
- Release date.
- Source category and tags from DLsite.
- Cover asset id.
- Preview image asset ids.
- Lightweight series or grouping text.

### InventoryItem

`InventoryItem` represents a concrete file or folder from NAS or staging.

Key fields:

- Source id, such as `nas-main`, `staging-upload`, or `watch-folder`.
- Current path.
- File name.
- Extension.
- File size.
- Modified time.
- Optional content hash.
- Legacy top-level folder, such as `ADV` or old `SIM+SLG`.
- Canonical category suggestion or confirmed category.
- Linked `GameWork` id, nullable.
- Version.
- Language.
- Patch location.
- Save location.
- Extracted flag.
- Downloaded flag.
- Organization status.
- Play status.
- Rating.
- Personal tags.
- Notes.

Organization status values:

- `pending`
- `matched`
- `confirmed`
- `ignored`

Play status values:

- `not_played`
- `want_to_play`
- `playing`
- `completed`
- `dropped`

### MetadataCandidate

`MetadataCandidate` stores source search results before the user confirms them.
Candidates never overwrite confirmed work data automatically.

Key fields:

- Metadata source name, initially `dlsite`.
- Source work id.
- Source URL.
- Query used.
- Confidence score or rank.
- Candidate title.
- Candidate circle or developer.
- Candidate cover URL.
- Normalized payload.
- Optional raw payload snapshot.

### ImportBatch and StagingItem

Imports enter app-managed staging before any NAS write.

Key fields:

- Staging path.
- Original filename.
- File size.
- Modified time.
- Optional hash.
- Suggested canonical category.
- Suggested filename.
- Suggested target path.
- Linked candidate or work id.
- Import status.
- Commit dry-run result.
- Conflict warnings.

Import status values:

- `staged`
- `reviewed`
- `ready_to_commit`
- `committed`
- `failed`

### Asset

`Asset` records cached external media.

Key fields:

- Source URL.
- Cache path.
- Media type.
- Width and height if known.
- Source attribution.
- Fetch status.
- Last fetched time.

## Library Identity

The scanner uses a mixed identity strategy:

- Primary identity is `source id + path`.
- File size and modified time detect normal changes.
- Hashing is deferred until needed for suspected duplicates, moves, or renames.

This avoids full reads of large zip/rar files on every scan while preserving a
path to better deduplication and move detection.

## Canonical Categories

Legacy NAS folders are preserved as source-location metadata only. They are not
the final taxonomy.

The first taxonomy is configurable and starts with:

- `Visual Novel`
- `Action`
- `RPG`
- `Simulation`
- `Strategy`
- `3D`
- `DLC/Patch`
- `Unsorted`

The scanner may use legacy folders as weak hints, but a legacy folder does not
automatically confirm a canonical category. For example, old `SIM+SLG` remains a
legacy location and can be split into `Simulation`, `Strategy`, `3D`, or other
custom categories.

## User Interface

The UI is library-first.

The default view is a cover-based game library. It should feel like a personal
game library first, not a spreadsheet. Management signals remain visible on each
card so cleanup work is still easy.

Primary navigation:

- `作品庫`: cover-first library browsing.
- `整理工作台`: pending inventory review queue.
- `匯入 staging`: uploaded and watched files waiting for review.
- `下載紀錄`: recent downloads.
- `設定`: sources, metadata adapters, categories, scan schedule, storage paths,
  and export/import.

Library center pane modes:

- `Cover`: default cover grid.
- `Table`: dense sorting and filtering view.
- `Review Queue`: focused cleanup queue for pending inventory.

Left sidebar filters:

- Canonical category.
- Personal tags.
- Quick filters, such as needs review, has DLsite, missing cover, downloaded,
  extracted, and favorites.
- Legacy location, such as `ADV`, `ACT`, `RPG`, old `SIM+SLG`, and `未分類`.

Right detail panel:

- Cover or preview image.
- Work title and source metadata.
- Inventory items linked to the work.
- Organization status.
- Play status.
- Rating.
- Personal tags and notes.
- Version and language.
- Patch and save locations.
- DLsite search and candidate selection.
- Download action.
- Organization dry-run preview.

## Workflows

### Library Scan

The user can trigger a manual scan. A configurable schedule can also run scans
periodically.

Scanning `/mnt/games`:

- Creates new inventory items.
- Updates changed file size and modified time.
- Marks missing files as absent instead of deleting their metadata immediately.
- Detects suspected duplicates or moved files.
- Preserves legacy path information.
- Suggests, but does not confirm, canonical categories.
- Records per-item warnings without failing the entire scan.

### DLsite Match

The user can search DLsite from an inventory item or game work.

The adapter builds queries from cleaned filenames, candidate titles, optional
circle names, and recognizable ids such as RJ codes. Search results are stored as
metadata candidates. The user chooses a candidate before importing normalized
metadata.

Confirmed imports may update:

- Title fields.
- Circle, developer, or publisher.
- DLsite work id.
- Source URL.
- Description.
- Release date.
- Tags and source category.
- Cover and preview image URLs.
- Cached image assets.

Existing user-confirmed metadata is not overwritten without explicit action.

### Download

Downloads are pure file downloads.

Supported first-version targets:

- `.zip`
- `.rar`

The server streams the file from the resolved inventory item path. Clients never
send arbitrary filesystem paths. Web clients use normal browser downloads.
Tauri desktop clients open a local save-location picker and then write the
stream to the selected local path.

Version 1 does not package folders into archives.

### Import and Staging

New games enter staging through:

- Browser upload.
- Server-side watch folder.

Each staged file receives a staging item. The user can search metadata sources,
assign canonical category, edit personal fields, and preview the final filename.

The system generates suggested names and target paths but requires manual
confirmation before any formal commit.

### Organization Preview and Future NAS Commit

During development, `/mnt/games` remains read-only to protect the original
archive.

The system still prepares for future controlled organization:

- Generate target path suggestions.
- Show source path, target path, category, filename, and file size.
- Detect target conflicts.
- Refuse overwrites by default.
- Keep staging originals until commit succeeds.
- Record commit failures.

Future writable NAS support must use an explicit `LibraryWriter` or
`CommitAdapter`. It must support dry-run first and require user confirmation.

Example future path shape:

```text
<canonical-category>/<circle-or-developer>/<title>/<title> <version>.<ext>
```

### Metadata Export and Import

The user can export metadata to a portable JSON snapshot.

Export includes:

- Game works.
- Inventory metadata and source ids.
- Metadata source provenance.
- Personal ratings, tags, statuses, and notes.
- Asset source URLs and cache metadata.
- Category definitions.
- Organization plans and staging metadata when requested.

Export does not include large game archives. Cached images are excluded by
default and can be rebuilt from source URLs.

Import validates the schema version and runs as a dry-run first. It reports
conflicts before writing to the current database. Import never writes to NAS
library paths.

## Safety and Error Handling

### NAS Protection

The system must not write to `/mnt/games` in the development configuration. All
write-capable organization features must be behind an explicit writable adapter.

NAS commit safety rules:

- Dry-run before write.
- No overwrite by default.
- Conflict blocks commit.
- Failed commit leaves source and staging files intact.
- Commit result is recorded.

### Path Safety

Download and file operations resolve paths through server-side item ids. API
clients must not be able to request arbitrary filesystem paths.

Storage adapters must enforce configured roots and reject path traversal.

### Scan Errors

Scan errors are item-scoped when possible:

- Permission failure.
- File disappeared during scan.
- File changed while scanning.
- Invalid or unsupported filename.
- Optional hash failure.

The scan summary reports created, updated, missing, changed, suspected duplicate,
and warning counts.

### DLsite Errors

The DLsite adapter supports:

- Timeout.
- Rate limit.
- Retry limit.
- User agent configuration.

Search failures do not block local metadata editing. Image download failures
leave source URLs intact and mark assets for retry.

### Download Errors

Download errors return clear API errors for:

- Missing inventory item.
- Missing file.
- Source unavailable.
- File changed since metadata was read.
- Unsupported directory download.

HTTP range support is optional for version 1, but the API design should not
preclude it.

### Deployment Limit

Version 1 has no login. It is intended only for a trusted home network or VPN.
It must not be exposed to the public internet. Public exposure requires adding
authentication, upload protections, CSRF/CORS policy, and rate limits first.

### Upload Limits

Upload and watch-folder imports only accept configured extensions. Initial
allowed extensions are `.zip`, `.rar`, and optionally `.7z`.

Upload size limits are configurable. The watch folder only scans configured
directories.

## Testing Strategy

### Backend Unit Tests

Cover:

- Filename cleanup and search-query generation.
- Version, language, bracket, RJ id, and extension parsing.
- Canonical category suggestion from weak hints.
- Inventory identity and changed-file detection.
- Suspected move, duplicate, and hash fallback logic.
- Organization plan generation.
- Target path conflict detection.
- Path safety and traversal rejection.

### Backend Integration Tests

Cover:

- PostgreSQL repository behavior.
- Future SQLite adapter CRUD and export/import if implemented.
- Portable JSON export/import validation and conflict reporting.
- Fixture library scan with legacy folders and zip/rar files.
- Missing, changed, duplicate, and moved file scenarios.
- Asset cache with mocked image downloads.
- Download endpoint with fixture files.
- Rejection of arbitrary path downloads.

### Metadata Source Tests

Cover:

- DLsite candidate normalization using fixtures.
- Candidate confirmation flow.
- Protection against overwriting confirmed fields unintentionally.
- Image URL preservation and cache retry state.

Live DLsite search tests should be manual or ignored by default so CI does not
depend on external site behavior.

### Frontend Tests

Cover:

- Library-first cover view.
- Filter sidebar.
- Detail panel.
- View switching between cover, table, and review queue.
- DLsite candidate selection.
- Import staging review.
- Organization preview.
- Download action branches for web and Tauri.

### Manual Acceptance

Before considering version 1 usable:

- Run scan against a read-only fixture library and confirm no source writes.
- Scan a fixture library shaped like the current NAS legacy folders.
- Match one item to a DLsite candidate and import metadata.
- Cache a cover image while retaining the source URL.
- Download a fixture zip/rar through the API.
- Upload a test zip to staging.
- Generate an organization dry-run plan with conflict detection.
- Simulate DLsite timeout and confirm local metadata editing still works.

## Open Decisions Deferred

- Exact canonical taxonomy names after real-world cleanup starts.
- Whether SQLite ships in version 1 or remains a later adapter.
- Whether HTTP range requests are included in version 1 downloads.
- Whether `.7z` is enabled by default.
- Exact NAS commit policy once a writable mount is available.

These decisions do not block the first implementation plan because the design
keeps them behind configuration or adapters.
