# HGame Manager Foundation Vertical Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first working vertical slice: Rust API, PostgreSQL schema, safe read-only library scan, file download, metadata skeleton, staging skeleton, export skeleton, and a library-first web UI.

**Architecture:** Centralized Rust server owns NAS, database, scan, metadata, staging, export, and download workflows. React/Vite web UI talks to Axum JSON APIs. Tauri packaging is scaffolded only after the web UI runs against the same API.

**Tech Stack:** Rust, Axum, Tokio, SQLx, PostgreSQL, Serde, React, TypeScript, Vite, TanStack Query, Tauri v2.

---

## Scope Check

The approved design covers several independently shippable subsystems. This plan implements the foundation vertical slice:

- Rust workspace and web workspace.
- Domain model for works, inventory, categories, assets, metadata candidates, and staging.
- PostgreSQL migration and repository basics.
- Read-only filesystem source rooted at a configured library path.
- Scanner for zip/rar inventory items.
- Download endpoint that resolves by item id, never by client-provided path.
- Organization dry-run path builder and conflict check.
- Portable JSON export shape.
- DLsite adapter interface with fixture-backed candidate tests.
- Staging upload/watch model and API shape.
- Library-first UI shell with cover/table/review modes.
- Minimal Tauri shell connected to the web build.

Separate later plans should complete:

- Live DLsite scraper hardening and rate-limit behavior.
- Full asset cache download and retry worker.
- RW NAS commit adapter.
- SQLite adapter.
- Full desktop download progress and native save dialog polish.
- k3s manifests and production ingress choices.

## File Structure

Create this structure:

```text
Cargo.toml
crates/hgame-domain/Cargo.toml
crates/hgame-domain/src/lib.rs
crates/hgame-domain/src/category.rs
crates/hgame-domain/src/inventory.rs
crates/hgame-domain/src/work.rs
crates/hgame-domain/src/asset.rs
crates/hgame-domain/src/metadata.rs
crates/hgame-domain/src/staging.rs
crates/hgame-app/Cargo.toml
crates/hgame-app/src/lib.rs
crates/hgame-app/src/config.rs
crates/hgame-app/src/storage.rs
crates/hgame-app/src/scanner.rs
crates/hgame-app/src/organization.rs
crates/hgame-app/src/export.rs
crates/hgame-app/src/metadata_source.rs
crates/hgame-app/src/staging.rs
crates/hgame-app/tests/scanner_fixture.rs
crates/hgame-server/Cargo.toml
crates/hgame-server/src/main.rs
crates/hgame-server/src/state.rs
crates/hgame-server/src/routes/mod.rs
crates/hgame-server/src/routes/health.rs
crates/hgame-server/src/routes/library.rs
crates/hgame-server/src/routes/download.rs
crates/hgame-server/src/routes/metadata.rs
crates/hgame-server/src/routes/staging.rs
crates/hgame-server/src/routes/export.rs
crates/hgame-server/migrations/0001_initial.sql
web/package.json
web/index.html
web/vite.config.ts
web/tsconfig.json
web/src/main.tsx
web/src/App.tsx
web/src/api/client.ts
web/src/api/types.ts
web/src/components/AppShell.tsx
web/src/components/LibraryGrid.tsx
web/src/components/LibraryTable.tsx
web/src/components/DetailPanel.tsx
web/src/components/ReviewQueue.tsx
web/src/components/StagingView.tsx
web/src/styles.css
web/src-tauri/Cargo.toml
web/src-tauri/build.rs
web/src-tauri/tauri.conf.json
web/src-tauri/src/main.rs
fixtures/library/ACT/action_game_v1.0.zip
fixtures/library/ADV/visual_novel.rar
fixtures/library/SIM+SLG/mixed_sim_strategy.rar
fixtures/library/未分類/unmatched.zip
```

Responsibilities:

- `hgame-domain`: pure types, enums, and validation helpers.
- `hgame-app`: application workflows and adapter traits.
- `hgame-server`: Axum HTTP API, PostgreSQL wiring, and streaming responses.
- `web`: shared SPA for browser and Tauri.
- `fixtures`: tiny text files using archive extensions for scan and download tests.

## Task 1: Workspace Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `crates/hgame-domain/Cargo.toml`
- Create: `crates/hgame-domain/src/lib.rs`
- Create: `crates/hgame-app/Cargo.toml`
- Create: `crates/hgame-app/src/lib.rs`
- Create: `crates/hgame-server/Cargo.toml`
- Create: `crates/hgame-server/src/main.rs`
- Create: `web/package.json`
- Create: `web/index.html`
- Create: `web/vite.config.ts`
- Create: `web/tsconfig.json`
- Create: `web/src/main.tsx`
- Create: `web/src/App.tsx`

- [ ] **Step 1: Create Rust workspace manifest**

Create `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
  "crates/hgame-domain",
  "crates/hgame-app",
  "crates/hgame-server"
]

[workspace.package]
edition = "2021"
license = "MIT"
version = "0.1.0"

[workspace.dependencies]
anyhow = "1"
async-trait = "0.1"
axum = "0.8"
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json", "migrate"] }
thiserror = "2"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["fs", "trace", "cors"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1", features = ["serde", "v4"] }
walkdir = "2"
```

- [ ] **Step 2: Create crate manifests**

Create `crates/hgame-domain/Cargo.toml`:

```toml
[package]
name = "hgame-domain"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
chrono.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
uuid.workspace = true
```

Create `crates/hgame-app/Cargo.toml`:

```toml
[package]
name = "hgame-app"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
anyhow.workspace = true
async-trait.workspace = true
chrono.workspace = true
hgame-domain = { path = "../hgame-domain" }
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tokio.workspace = true
uuid.workspace = true
walkdir.workspace = true
```

Create `crates/hgame-server/Cargo.toml`:

```toml
[package]
name = "hgame-server"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
anyhow.workspace = true
axum.workspace = true
chrono.workspace = true
hgame-app = { path = "../hgame-app" }
hgame-domain = { path = "../hgame-domain" }
serde.workspace = true
serde_json.workspace = true
sqlx.workspace = true
tokio.workspace = true
tower-http.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
uuid.workspace = true
```

- [ ] **Step 3: Add minimal Rust entry points**

Create `crates/hgame-domain/src/lib.rs`:

```rust
pub mod asset;
pub mod category;
pub mod inventory;
pub mod metadata;
pub mod staging;
pub mod work;
```

Create `crates/hgame-app/src/lib.rs`:

```rust
pub mod config;
pub mod export;
pub mod metadata_source;
pub mod organization;
pub mod scanner;
pub mod staging;
pub mod storage;
```

Create `crates/hgame-server/src/main.rs`:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    println!("hgame-server scaffold");
    Ok(())
}
```

- [ ] **Step 4: Add minimal web scaffold**

Create `web/package.json`:

```json
{
  "name": "hgame-manager-web",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview",
    "test": "vitest run"
  },
  "dependencies": {
    "@tanstack/react-query": "^5.80.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.3.0",
    "typescript": "^5.8.0",
    "vite": "^6.0.0",
    "vitest": "^3.0.0"
  }
}
```

Create `web/index.html`:

```html
<div id="root"></div>
<script type="module" src="/src/main.tsx"></script>
```

Create `web/vite.config.ts`:

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      "/api": "http://localhost:3000"
    }
  }
});
```

Create `web/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["DOM", "DOM.Iterable", "ES2022"],
    "allowJs": false,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "forceConsistentCasingInFileNames": true,
    "module": "ESNext",
    "moduleResolution": "Node",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx"
  },
  "include": ["src"]
}
```

Create `web/src/main.tsx`:

```tsx
import React from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { App } from "./App";
import "./styles.css";

const queryClient = new QueryClient();

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>
);
```

Create `web/src/App.tsx`:

```tsx
export function App() {
  return <main>HGame Manager</main>;
}
```

- [ ] **Step 5: Verify scaffold**

Run:

```bash
cargo check
```

Expected:

```text
Finished `dev` profile
```

Run:

```bash
cd web && npm install && npm run build
```

Expected:

```text
vite v
✓ built
```

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates web
git commit -m "chore: scaffold hgame manager workspace"
```

## Task 2: Domain Models

**Files:**
- Create: `crates/hgame-domain/src/category.rs`
- Create: `crates/hgame-domain/src/inventory.rs`
- Create: `crates/hgame-domain/src/work.rs`
- Create: `crates/hgame-domain/src/asset.rs`
- Create: `crates/hgame-domain/src/metadata.rs`
- Create: `crates/hgame-domain/src/staging.rs`

- [ ] **Step 1: Write category tests**

Create `crates/hgame-domain/src/category.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimaryCategory(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenreFacet(String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_category_names() {
        let category = PrimaryCategory::new("  Simulation  ").unwrap();
        assert_eq!(category.as_str(), "Simulation");
    }

    #[test]
    fn rejects_empty_category_names() {
        assert!(PrimaryCategory::new(" ").is_err());
    }

    #[test]
    fn supports_mixed_genre_facets() {
        let facets = vec![
            GenreFacet::new("Simulation").unwrap(),
            GenreFacet::new("Strategy").unwrap(),
        ];
        assert_eq!(facets[0].as_str(), "Simulation");
        assert_eq!(facets[1].as_str(), "Strategy");
    }
}
```

- [ ] **Step 2: Run category tests to verify failure**

Run:

```bash
cargo test -p hgame-domain category
```

Expected: compile failure because `new` and `as_str` are not defined.

- [ ] **Step 3: Implement category value objects**

Replace `crates/hgame-domain/src/category.rs` with:

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CategoryError {
    #[error("category name cannot be empty")]
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PrimaryCategory(String);

impl PrimaryCategory {
    pub fn new(value: impl Into<String>) -> Result<Self, CategoryError> {
        let value = value.into().trim().to_string();
        if value.is_empty() {
            return Err(CategoryError::Empty);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GenreFacet(String);

impl GenreFacet {
    pub fn new(value: impl Into<String>) -> Result<Self, CategoryError> {
        let value = value.into().trim().to_string();
        if value.is_empty() {
            return Err(CategoryError::Empty);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

- [ ] **Step 4: Add remaining domain types**

Create `crates/hgame-domain/src/inventory.rs`:

```rust
use crate::category::{GenreFacet, PrimaryCategory};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganizationStatus {
    Pending,
    Matched,
    Confirmed,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayStatus {
    NotPlayed,
    WantToPlay,
    Playing,
    Completed,
    Dropped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InventoryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: Uuid,
    pub source_id: String,
    pub path: PathBuf,
    pub file_name: String,
    pub extension: Option<String>,
    pub kind: InventoryKind,
    pub file_size: u64,
    pub modified_at: DateTime<Utc>,
    pub content_hash: Option<String>,
    pub legacy_location: Option<String>,
    pub primary_category: Option<PrimaryCategory>,
    pub genre_facets: Vec<GenreFacet>,
    pub game_work_id: Option<Uuid>,
    pub version: Option<String>,
    pub language: Option<String>,
    pub patch_location: Option<String>,
    pub save_location: Option<String>,
    pub extracted: bool,
    pub downloaded: bool,
    pub organization_status: OrganizationStatus,
    pub play_status: PlayStatus,
    pub rating: Option<u8>,
    pub personal_tags: Vec<String>,
    pub notes: Option<String>,
}
```

Create `crates/hgame-domain/src/work.rs`:

```rust
use crate::category::{GenreFacet, PrimaryCategory};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameWork {
    pub id: Uuid,
    pub canonical_title: String,
    pub original_title: Option<String>,
    pub display_title: String,
    pub circle: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub source_urls: Vec<String>,
    pub dlsite_work_id: Option<String>,
    pub description: Option<String>,
    pub release_date: Option<NaiveDate>,
    pub source_tags: Vec<String>,
    pub primary_category: Option<PrimaryCategory>,
    pub genre_facets: Vec<GenreFacet>,
    pub cover_asset_id: Option<Uuid>,
    pub preview_asset_ids: Vec<Uuid>,
    pub series: Option<String>,
}
```

Create `crates/hgame-domain/src/asset.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetFetchStatus {
    Pending,
    Cached,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: Uuid,
    pub source_url: String,
    pub cache_path: PathBuf,
    pub media_type: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub source_attribution: Option<String>,
    pub fetch_status: AssetFetchStatus,
    pub last_fetched_at: Option<DateTime<Utc>>,
}
```

Create `crates/hgame-domain/src/metadata.rs`:

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataCandidate {
    pub id: Uuid,
    pub source_name: String,
    pub source_work_id: String,
    pub source_url: String,
    pub query_used: String,
    pub rank: i32,
    pub title: String,
    pub circle: Option<String>,
    pub cover_url: Option<String>,
    pub normalized_payload: serde_json::Value,
}
```

Create `crates/hgame-domain/src/staging.rs`:

```rust
use crate::category::{GenreFacet, PrimaryCategory};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportStatus {
    Staged,
    Reviewed,
    ReadyToCommit,
    Committed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagingItem {
    pub id: Uuid,
    pub staging_path: PathBuf,
    pub original_filename: String,
    pub file_size: u64,
    pub modified_at: DateTime<Utc>,
    pub content_hash: Option<String>,
    pub suggested_primary_category: Option<PrimaryCategory>,
    pub suggested_genre_facets: Vec<GenreFacet>,
    pub suggested_filename: Option<String>,
    pub suggested_target_path: Option<PathBuf>,
    pub linked_candidate_id: Option<Uuid>,
    pub linked_work_id: Option<Uuid>,
    pub import_status: ImportStatus,
    pub conflict_warnings: Vec<String>,
}
```

- [ ] **Step 5: Run tests**

Run:

```bash
cargo test -p hgame-domain
```

Expected:

```text
test result: ok
```

- [ ] **Step 6: Commit**

```bash
git add crates/hgame-domain
git commit -m "feat: add hgame domain model"
```

## Task 3: Configuration and Safe Storage Roots

**Files:**
- Create: `crates/hgame-app/src/config.rs`
- Create: `crates/hgame-app/src/storage.rs`

- [ ] **Step 1: Write storage root tests**

Create `crates/hgame-app/src/storage.rs`:

```rust
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct StorageRoot {
    root: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_child_inside_root() {
        let root = StorageRoot::new("/mnt/games");
        let resolved = root.resolve_relative("ADV/game.zip").unwrap();
        assert_eq!(resolved, PathBuf::from("/mnt/games/ADV/game.zip"));
    }

    #[test]
    fn rejects_parent_traversal() {
        let root = StorageRoot::new("/mnt/games");
        assert!(root.resolve_relative("../secret.zip").is_err());
    }
}
```

- [ ] **Step 2: Run storage tests to verify failure**

Run:

```bash
cargo test -p hgame-app storage
```

Expected: compile failure because `StorageRoot::new` and `resolve_relative` are not defined.

- [ ] **Step 3: Implement safe storage roots**

Replace `crates/hgame-app/src/storage.rs` with:

```rust
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("relative path cannot be empty")]
    EmptyPath,
    #[error("path traversal is not allowed")]
    Traversal,
    #[error("absolute client paths are not allowed")]
    AbsolutePath,
}

#[derive(Debug, Clone)]
pub struct StorageRoot {
    root: PathBuf,
}

impl StorageRoot {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn resolve_relative(&self, relative: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
        let relative = relative.as_ref();
        if relative.as_os_str().is_empty() {
            return Err(StorageError::EmptyPath);
        }
        if relative.is_absolute() {
            return Err(StorageError::AbsolutePath);
        }
        for component in relative.components() {
            match component {
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(StorageError::Traversal);
                }
                Component::CurDir | Component::Normal(_) => {}
            }
        }
        Ok(self.root.join(relative))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_child_inside_root() {
        let root = StorageRoot::new("/mnt/games");
        let resolved = root.resolve_relative("ADV/game.zip").unwrap();
        assert_eq!(resolved, PathBuf::from("/mnt/games/ADV/game.zip"));
    }

    #[test]
    fn rejects_parent_traversal() {
        let root = StorageRoot::new("/mnt/games");
        assert!(root.resolve_relative("../secret.zip").is_err());
    }

    #[test]
    fn rejects_absolute_path() {
        let root = StorageRoot::new("/mnt/games");
        assert!(root.resolve_relative("/etc/passwd").is_err());
    }
}
```

- [ ] **Step 4: Add app config**

Create `crates/hgame-app/src/config.rs`:

```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub library_root: PathBuf,
    pub staging_root: PathBuf,
    pub asset_cache_root: PathBuf,
    pub bind_addr: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/hgame_manager".to_string()),
            library_root: std::env::var("HGAME_LIBRARY_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/mnt/games")),
            staging_root: std::env::var("HGAME_STAGING_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("./var/staging")),
            asset_cache_root: std::env::var("HGAME_ASSET_CACHE_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("./var/assets")),
            bind_addr: std::env::var("HGAME_BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:3000".to_string()),
        }
    }
}
```

- [ ] **Step 5: Run tests**

Run:

```bash
cargo test -p hgame-app storage
```

Expected:

```text
test result: ok
```

- [ ] **Step 6: Commit**

```bash
git add crates/hgame-app/src/config.rs crates/hgame-app/src/storage.rs
git commit -m "feat: add safe storage root handling"
```

## Task 4: PostgreSQL Schema

**Files:**
- Create: `crates/hgame-server/migrations/0001_initial.sql`
- Create: `crates/hgame-server/src/state.rs`

- [ ] **Step 1: Create migration**

Create `crates/hgame-server/migrations/0001_initial.sql`:

```sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE game_works (
    id uuid PRIMARY KEY,
    canonical_title text NOT NULL,
    original_title text,
    display_title text NOT NULL,
    circle text,
    developer text,
    publisher text,
    source_urls jsonb NOT NULL DEFAULT '[]',
    dlsite_work_id text,
    description text,
    release_date date,
    source_tags jsonb NOT NULL DEFAULT '[]',
    primary_category text,
    genre_facets jsonb NOT NULL DEFAULT '[]',
    cover_asset_id uuid,
    preview_asset_ids jsonb NOT NULL DEFAULT '[]',
    series text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE inventory_items (
    id uuid PRIMARY KEY,
    source_id text NOT NULL,
    path text NOT NULL,
    file_name text NOT NULL,
    extension text,
    kind text NOT NULL,
    file_size bigint NOT NULL,
    modified_at timestamptz NOT NULL,
    content_hash text,
    legacy_location text,
    primary_category text,
    genre_facets jsonb NOT NULL DEFAULT '[]',
    game_work_id uuid REFERENCES game_works(id),
    version text,
    language text,
    patch_location text,
    save_location text,
    extracted boolean NOT NULL DEFAULT false,
    downloaded boolean NOT NULL DEFAULT false,
    organization_status text NOT NULL,
    play_status text NOT NULL,
    rating smallint,
    personal_tags jsonb NOT NULL DEFAULT '[]',
    notes text,
    missing boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(source_id, path)
);

CREATE TABLE metadata_candidates (
    id uuid PRIMARY KEY,
    source_name text NOT NULL,
    source_work_id text NOT NULL,
    source_url text NOT NULL,
    query_used text NOT NULL,
    rank integer NOT NULL,
    title text NOT NULL,
    circle text,
    cover_url text,
    normalized_payload jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE assets (
    id uuid PRIMARY KEY,
    source_url text NOT NULL,
    cache_path text NOT NULL,
    media_type text,
    width integer,
    height integer,
    source_attribution text,
    fetch_status text NOT NULL,
    last_fetched_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE staging_items (
    id uuid PRIMARY KEY,
    staging_path text NOT NULL,
    original_filename text NOT NULL,
    file_size bigint NOT NULL,
    modified_at timestamptz NOT NULL,
    content_hash text,
    suggested_primary_category text,
    suggested_genre_facets jsonb NOT NULL DEFAULT '[]',
    suggested_filename text,
    suggested_target_path text,
    linked_candidate_id uuid REFERENCES metadata_candidates(id),
    linked_work_id uuid REFERENCES game_works(id),
    import_status text NOT NULL,
    conflict_warnings jsonb NOT NULL DEFAULT '[]',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX inventory_items_status_idx ON inventory_items(organization_status);
CREATE INDEX inventory_items_source_idx ON inventory_items(source_id);
CREATE INDEX game_works_title_idx ON game_works(display_title);
```

- [ ] **Step 2: Add database state**

Create `crates/hgame-server/src/state.rs`:

```rust
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

impl AppState {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await?;

        sqlx::migrate!("./migrations").run(&db).await?;

        Ok(Self { db })
    }
}
```

- [ ] **Step 3: Wire server startup to state**

Replace `crates/hgame-server/src/main.rs` with:

```rust
mod state;

use hgame_app::config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env();
    let _state = state::AppState::connect(&config.database_url).await?;
    tracing::info!("hgame-server initialized");
    Ok(())
}
```

- [ ] **Step 4: Verify migration with local Postgres**

Run:

```bash
export DATABASE_URL=postgres://postgres:postgres@localhost/hgame_manager
cargo run -p hgame-server
```

Expected:

```text
hgame-server initialized
```

- [ ] **Step 5: Commit**

```bash
git add crates/hgame-server
git commit -m "feat: add initial postgres schema"
```

## Task 5: Read-Only Scanner

**Files:**
- Create: `crates/hgame-app/src/scanner.rs`
- Create: `crates/hgame-app/tests/scanner_fixture.rs`
- Create: fixture files under `fixtures/library`

- [ ] **Step 1: Create fixture archive files**

Create these files as small text fixtures:

```text
fixtures/library/ACT/action_game_v1.0.zip
fixtures/library/ADV/visual_novel.rar
fixtures/library/SIM+SLG/mixed_sim_strategy.rar
fixtures/library/未分類/unmatched.zip
```

Each file content can be:

```text
fixture archive content
```

- [ ] **Step 2: Write scanner integration test**

Create `crates/hgame-app/tests/scanner_fixture.rs`:

```rust
use hgame_app::scanner::{scan_library, ScanOptions};
use std::path::PathBuf;

#[tokio::test]
async fn scans_zip_and_rar_files_with_legacy_location() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/library");

    let result = scan_library(ScanOptions {
        source_id: "fixture".to_string(),
        root,
    })
    .await
    .unwrap();

    assert_eq!(result.items.len(), 4);
    assert!(result.items.iter().any(|item| {
        item.file_name == "mixed_sim_strategy.rar"
            && item.legacy_location.as_deref() == Some("SIM+SLG")
            && item.extension.as_deref() == Some("rar")
    }));
}
```

- [ ] **Step 3: Run scanner test to verify failure**

Run:

```bash
cargo test -p hgame-app --test scanner_fixture
```

Expected: compile failure because `scan_library` and `ScanOptions` are not defined.

- [ ] **Step 4: Implement scanner**

Create `crates/hgame-app/src/scanner.rs`:

```rust
use chrono::{DateTime, Utc};
use hgame_domain::inventory::{
    InventoryItem, InventoryKind, OrganizationStatus, PlayStatus,
};
use std::path::{Path, PathBuf};
use tokio::task;
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub source_id: String,
    pub root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub items: Vec<InventoryItem>,
    pub warnings: Vec<String>,
}

pub async fn scan_library(options: ScanOptions) -> anyhow::Result<ScanResult> {
    task::spawn_blocking(move || scan_library_blocking(options)).await?
}

fn scan_library_blocking(options: ScanOptions) -> anyhow::Result<ScanResult> {
    let mut items = Vec::new();
    let mut warnings = Vec::new();

    for entry in WalkDir::new(&options.root).min_depth(1) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                warnings.push(err.to_string());
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase());

        if !matches!(extension.as_deref(), Some("zip" | "rar")) {
            continue;
        }

        let metadata = entry.metadata()?;
        let modified_at: DateTime<Utc> = metadata.modified()?.into();
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        let legacy_location = legacy_location(&options.root, path);

        items.push(InventoryItem {
            id: Uuid::new_v4(),
            source_id: options.source_id.clone(),
            path: path.to_path_buf(),
            file_name,
            extension,
            kind: InventoryKind::File,
            file_size: metadata.len(),
            modified_at,
            content_hash: None,
            legacy_location,
            primary_category: None,
            genre_facets: Vec::new(),
            game_work_id: None,
            version: None,
            language: None,
            patch_location: None,
            save_location: None,
            extracted: false,
            downloaded: false,
            organization_status: OrganizationStatus::Pending,
            play_status: PlayStatus::NotPlayed,
            rating: None,
            personal_tags: Vec::new(),
            notes: None,
        });
    }

    Ok(ScanResult { items, warnings })
}

fn legacy_location(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| relative.components().next())
        .and_then(|component| component.as_os_str().to_str())
        .map(ToOwned::to_owned)
}
```

- [ ] **Step 5: Run scanner test**

Run:

```bash
cargo test -p hgame-app --test scanner_fixture
```

Expected:

```text
test scans_zip_and_rar_files_with_legacy_location ... ok
```

- [ ] **Step 6: Commit**

```bash
git add crates/hgame-app/src/scanner.rs crates/hgame-app/tests/scanner_fixture.rs fixtures
git commit -m "feat: scan read-only library files"
```

## Task 6: Organization Planner

**Files:**
- Create: `crates/hgame-app/src/organization.rs`

- [ ] **Step 1: Write planner tests**

Create `crates/hgame-app/src/organization.rs`:

```rust
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_primary_category_path() {
        let plan = build_target_path("RPG", "Alicesoft", "Sample Game", Some("v1.02"), "zip");
        assert_eq!(
            plan,
            PathBuf::from("RPG/Alicesoft/Sample Game/Sample Game v1.02.zip")
        );
    }

    #[test]
    fn sanitizes_path_separators() {
        let plan = build_target_path("RPG", "Circle/Name", "Game:Name", None, "rar");
        assert_eq!(plan, PathBuf::from("RPG/Circle_Name/Game_Name/Game_Name.rar"));
    }
}
```

- [ ] **Step 2: Run planner tests to verify failure**

Run:

```bash
cargo test -p hgame-app organization
```

Expected: compile failure because `build_target_path` is not defined.

- [ ] **Step 3: Implement planner**

Replace `crates/hgame-app/src/organization.rs` with:

```rust
use std::path::PathBuf;

pub fn build_target_path(
    primary_category: &str,
    circle_or_developer: &str,
    title: &str,
    version: Option<&str>,
    extension: &str,
) -> PathBuf {
    let safe_category = sanitize_segment(primary_category);
    let safe_owner = sanitize_segment(circle_or_developer);
    let safe_title = sanitize_segment(title);
    let safe_extension = extension.trim_start_matches('.');

    let filename = match version {
        Some(version) if !version.trim().is_empty() => {
            format!("{} {}.{}", safe_title, sanitize_segment(version), safe_extension)
        }
        _ => format!("{}.{}", safe_title, safe_extension),
    };

    PathBuf::from(safe_category)
        .join(safe_owner)
        .join(&safe_title)
        .join(filename)
}

fn sanitize_segment(value: &str) -> String {
    let replaced = value
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>();
    let trimmed = replaced.trim();
    if trimmed.is_empty() {
        "Unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_primary_category_path() {
        let plan = build_target_path("RPG", "Alicesoft", "Sample Game", Some("v1.02"), "zip");
        assert_eq!(
            plan,
            PathBuf::from("RPG/Alicesoft/Sample Game/Sample Game v1.02.zip")
        );
    }

    #[test]
    fn sanitizes_path_separators() {
        let plan = build_target_path("RPG", "Circle/Name", "Game:Name", None, "rar");
        assert_eq!(plan, PathBuf::from("RPG/Circle_Name/Game_Name/Game_Name.rar"));
    }
}
```

- [ ] **Step 4: Run planner tests**

Run:

```bash
cargo test -p hgame-app organization
```

Expected:

```text
test result: ok
```

- [ ] **Step 5: Commit**

```bash
git add crates/hgame-app/src/organization.rs
git commit -m "feat: add organization path planner"
```

## Task 7: Axum API Shell

**Files:**
- Create: `crates/hgame-server/src/routes/mod.rs`
- Create: `crates/hgame-server/src/routes/health.rs`
- Create: `crates/hgame-server/src/routes/library.rs`
- Create: `crates/hgame-server/src/routes/download.rs`
- Create: `crates/hgame-server/src/routes/metadata.rs`
- Create: `crates/hgame-server/src/routes/staging.rs`
- Create: `crates/hgame-server/src/routes/export.rs`
- Modify: `crates/hgame-server/src/main.rs`

- [ ] **Step 1: Create route modules**

Create `crates/hgame-server/src/routes/mod.rs`:

```rust
pub mod download;
pub mod export;
pub mod health;
pub mod library;
pub mod metadata;
pub mod staging;

use crate::state::AppState;
use axum::{routing::get, Router};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health::health))
        .nest("/api/library", library::router())
        .nest("/api/downloads", download::router())
        .nest("/api/metadata", metadata::router())
        .nest("/api/staging", staging::router())
        .nest("/api/export", export::router())
        .with_state(state)
}
```

Create `crates/hgame-server/src/routes/health.rs`:

```rust
use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
```

Create initial routers with empty-list responses:

```rust
use crate::state::AppState;
use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct EmptyList<T> {
    items: Vec<T>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list))
}

async fn list() -> Json<EmptyList<serde_json::Value>> {
    Json(EmptyList { items: Vec::new() })
}
```

Use that content for:

```text
crates/hgame-server/src/routes/library.rs
crates/hgame-server/src/routes/metadata.rs
crates/hgame-server/src/routes/staging.rs
crates/hgame-server/src/routes/export.rs
```

Create `crates/hgame-server/src/routes/download.rs`:

```rust
use crate::state::AppState;
use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct DownloadStatus {
    status: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(index))
}

async fn index() -> Json<DownloadStatus> {
    Json(DownloadStatus { status: "download routes ready" })
}
```

- [ ] **Step 2: Wire main server**

Replace `crates/hgame-server/src/main.rs` with:

```rust
mod routes;
mod state;

use hgame_app::config::AppConfig;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env();
    let state = state::AppState::connect(&config.database_url).await?;
    let app = routes::router(state).layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 3: Run server check**

Run:

```bash
cargo check -p hgame-server
```

Expected:

```text
Finished `dev` profile
```

- [ ] **Step 4: Commit**

```bash
git add crates/hgame-server/src
git commit -m "feat: add axum api shell"
```

## Task 8: Download Endpoint Path Safety

**Files:**
- Modify: `crates/hgame-server/src/routes/download.rs`
- Modify: `crates/hgame-server/src/state.rs`

- [ ] **Step 1: Add server state fields**

Modify `crates/hgame-server/src/state.rs`:

```rust
use hgame_app::storage::StorageRoot;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub library_root: StorageRoot,
}

impl AppState {
    pub async fn connect(database_url: &str, library_root: StorageRoot) -> anyhow::Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await?;

        sqlx::migrate!("./migrations").run(&db).await?;

        Ok(Self { db, library_root })
    }
}
```

Modify `crates/hgame-server/src/main.rs` state creation:

```rust
let state = state::AppState::connect(
    &config.database_url,
    hgame_app::storage::StorageRoot::new(config.library_root.clone()),
)
.await?;
```

- [ ] **Step 2: Add item-id download route**

Replace `crates/hgame-server/src/routes/download.rs` with:

```rust
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use sqlx::Row;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/{id}", get(download_item))
}

async fn download_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    let row = sqlx::query("SELECT path, file_name FROM inventory_items WHERE id = $1")
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(row) = row else {
        return Err(StatusCode::NOT_FOUND);
    };

    let item_path: String = row.get("path");
    let file_name: String = row.get("file_name");

    let relative = std::path::Path::new(&item_path)
        .strip_prefix(state.library_root.root())
        .map_err(|_| StatusCode::FORBIDDEN)?;
    let resolved = state
        .library_root
        .resolve_relative(relative)
        .map_err(|_| StatusCode::FORBIDDEN)?;

    let file = File::open(resolved).await.map_err(|_| StatusCode::NOT_FOUND)?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let content_disposition = format!("attachment; filename=\"{}\"", file_name);

    Ok(([
        (header::CONTENT_TYPE, "application/octet-stream".to_string()),
        (header::CONTENT_DISPOSITION, content_disposition),
    ], body)
        .into_response())
}
```

- [ ] **Step 3: Add missing dependency**

Modify `crates/hgame-server/Cargo.toml` dependencies:

```toml
tokio-util = { version = "0.7", features = ["io"] }
```

- [ ] **Step 4: Run check**

Run:

```bash
cargo check -p hgame-server
```

Expected:

```text
Finished `dev` profile
```

- [ ] **Step 5: Commit**

```bash
git add crates/hgame-server
git commit -m "feat: stream downloads by inventory id"
```

## Task 9: Metadata Source Interface

**Files:**
- Create: `crates/hgame-app/src/metadata_source.rs`

- [ ] **Step 1: Write fixture metadata test**

Create `crates/hgame-app/src/metadata_source.rs`:

```rust
use hgame_domain::metadata::MetadataCandidate;

#[async_trait::async_trait]
pub trait MetadataSource {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<MetadataCandidate>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fixture_source_returns_ranked_candidate() {
        let source = FixtureMetadataSource;
        let result = source.search("sample").await.unwrap();
        assert_eq!(result[0].source_name, "dlsite-fixture");
        assert_eq!(result[0].rank, 1);
    }
}
```

- [ ] **Step 2: Run metadata test to verify failure**

Run:

```bash
cargo test -p hgame-app metadata_source
```

Expected: compile failure because `FixtureMetadataSource` is not defined.

- [ ] **Step 3: Implement fixture source**

Replace `crates/hgame-app/src/metadata_source.rs` with:

```rust
use hgame_domain::metadata::MetadataCandidate;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait MetadataSource {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<MetadataCandidate>>;
}

pub struct FixtureMetadataSource;

#[async_trait::async_trait]
impl MetadataSource for FixtureMetadataSource {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<MetadataCandidate>> {
        Ok(vec![MetadataCandidate {
            id: Uuid::new_v4(),
            source_name: "dlsite-fixture".to_string(),
            source_work_id: "RJ000001".to_string(),
            source_url: "https://www.dlsite.com/maniax/work/=/product_id/RJ000001.html".to_string(),
            query_used: query.to_string(),
            rank: 1,
            title: "Sample Candidate".to_string(),
            circle: Some("Sample Circle".to_string()),
            cover_url: Some("https://example.invalid/cover.jpg".to_string()),
            normalized_payload: serde_json::json!({
                "title": "Sample Candidate",
                "circle": "Sample Circle"
            }),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fixture_source_returns_ranked_candidate() {
        let source = FixtureMetadataSource;
        let result = source.search("sample").await.unwrap();
        assert_eq!(result[0].source_name, "dlsite-fixture");
        assert_eq!(result[0].rank, 1);
    }
}
```

- [ ] **Step 4: Run metadata tests**

Run:

```bash
cargo test -p hgame-app metadata_source
```

Expected:

```text
test result: ok
```

- [ ] **Step 5: Commit**

```bash
git add crates/hgame-app/src/metadata_source.rs
git commit -m "feat: add metadata source interface"
```

## Task 10: Export Snapshot Shape

**Files:**
- Create: `crates/hgame-app/src/export.rs`

- [ ] **Step 1: Write export serialization test**

Create `crates/hgame-app/src/export.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportSnapshot {
    pub schema_version: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_has_schema_version() {
        let snapshot = ExportSnapshot::empty();
        let json = serde_json::to_value(snapshot).unwrap();
        assert_eq!(json["schema_version"], 1);
    }
}
```

- [ ] **Step 2: Run export test to verify failure**

Run:

```bash
cargo test -p hgame-app export
```

Expected: compile failure because `ExportSnapshot::empty` is not defined.

- [ ] **Step 3: Implement export snapshot**

Replace `crates/hgame-app/src/export.rs` with:

```rust
use hgame_domain::{
    asset::Asset, inventory::InventoryItem, metadata::MetadataCandidate,
    staging::StagingItem, work::GameWork,
};
use serde::{Deserialize, Serialize};

pub const EXPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportSnapshot {
    pub schema_version: u32,
    pub game_works: Vec<GameWork>,
    pub inventory_items: Vec<InventoryItem>,
    pub metadata_candidates: Vec<MetadataCandidate>,
    pub assets: Vec<Asset>,
    pub staging_items: Vec<StagingItem>,
    pub category_definitions: Vec<String>,
}

impl ExportSnapshot {
    pub fn empty() -> Self {
        Self {
            schema_version: EXPORT_SCHEMA_VERSION,
            game_works: Vec::new(),
            inventory_items: Vec::new(),
            metadata_candidates: Vec::new(),
            assets: Vec::new(),
            staging_items: Vec::new(),
            category_definitions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_has_schema_version() {
        let snapshot = ExportSnapshot::empty();
        let json = serde_json::to_value(snapshot).unwrap();
        assert_eq!(json["schema_version"], 1);
    }
}
```

- [ ] **Step 4: Run export tests**

Run:

```bash
cargo test -p hgame-app export
```

Expected:

```text
test result: ok
```

- [ ] **Step 5: Commit**

```bash
git add crates/hgame-app/src/export.rs
git commit -m "feat: add portable export snapshot"
```

## Task 11: Web API Types and Client

**Files:**
- Create: `web/src/api/types.ts`
- Create: `web/src/api/client.ts`

- [ ] **Step 1: Add API types**

Create `web/src/api/types.ts`:

```ts
export type OrganizationStatus = "pending" | "matched" | "confirmed" | "ignored";
export type PlayStatus = "not_played" | "want_to_play" | "playing" | "completed" | "dropped";

export interface InventoryItem {
  id: string;
  sourceId: string;
  fileName: string;
  legacyLocation: string | null;
  primaryCategory: string | null;
  genreFacets: string[];
  organizationStatus: OrganizationStatus;
  playStatus: PlayStatus;
  rating: number | null;
  version: string | null;
  language: string | null;
  notes: string | null;
}

export interface LibraryResponse {
  items: InventoryItem[];
}
```

- [ ] **Step 2: Add API client**

Create `web/src/api/client.ts`:

```ts
import type { LibraryResponse } from "./types";

const API_BASE = import.meta.env.VITE_API_BASE ?? "";

export async function fetchLibrary(): Promise<LibraryResponse> {
  const response = await fetch(`${API_BASE}/api/library/`);
  if (!response.ok) {
    throw new Error(`Library request failed: ${response.status}`);
  }
  return response.json();
}

export function downloadUrl(itemId: string): string {
  return `${API_BASE}/api/downloads/${itemId}`;
}
```

- [ ] **Step 3: Run TypeScript check**

Run:

```bash
cd web && npm run build
```

Expected:

```text
✓ built
```

- [ ] **Step 4: Commit**

```bash
git add web/src/api
git commit -m "feat: add web api client"
```

## Task 12: Library-First Web UI Shell

**Files:**
- Modify: `web/src/App.tsx`
- Create: `web/src/components/AppShell.tsx`
- Create: `web/src/components/LibraryGrid.tsx`
- Create: `web/src/components/LibraryTable.tsx`
- Create: `web/src/components/ReviewQueue.tsx`
- Create: `web/src/components/DetailPanel.tsx`
- Create: `web/src/components/StagingView.tsx`
- Create: `web/src/styles.css`

- [ ] **Step 1: Add shell components**

Create `web/src/components/AppShell.tsx`:

```tsx
import type { ReactNode } from "react";

interface AppShellProps {
  children: ReactNode;
  detail: ReactNode;
}

export function AppShell({ children, detail }: AppShellProps) {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">HG</div>
        <nav>
          <button className="active">作品庫</button>
          <button>整理工作台</button>
          <button>匯入 staging</button>
          <button>下載紀錄</button>
          <button>設定</button>
        </nav>
        <section>
          <h3>Primary category</h3>
          <div className="chips">
            {["Visual Novel", "Action", "RPG", "Simulation", "Strategy", "3D"].map((item) => (
              <span className="chip" key={item}>{item}</span>
            ))}
          </div>
        </section>
        <section>
          <h3>Genre facets</h3>
          <div className="chips">
            {["Needs review", "Has DLsite", "Missing cover", "Downloaded"].map((item) => (
              <span className="chip" key={item}>{item}</span>
            ))}
          </div>
        </section>
        <section>
          <h3>Legacy location</h3>
          <div className="chips">
            {["ADV", "ACT", "RPG", "舊 SIM+SLG", "未分類"].map((item) => (
              <span className="chip muted" key={item}>{item}</span>
            ))}
          </div>
        </section>
      </aside>
      <main className="main-pane">{children}</main>
      <aside className="detail-pane">{detail}</aside>
    </div>
  );
}
```

Create `web/src/components/LibraryGrid.tsx`:

```tsx
import type { InventoryItem } from "../api/types";

interface LibraryGridProps {
  items: InventoryItem[];
  selectedId: string | null;
  onSelect: (item: InventoryItem) => void;
}

export function LibraryGrid({ items, selectedId, onSelect }: LibraryGridProps) {
  return (
    <div className="library-grid">
      {items.map((item, index) => (
        <button
          className={`game-card ${selectedId === item.id ? "selected" : ""}`}
          key={item.id}
          onClick={() => onSelect(item)}
        >
          <div className={`cover cover-${index % 6}`}>
            <span>{item.organizationStatus}</span>
          </div>
          <strong>{item.fileName}</strong>
          <small>{item.primaryCategory ?? "Unsorted"} · {item.legacyLocation ?? "no legacy"}</small>
          <div className="mini-tags">
            {item.genreFacets.map((facet) => <span key={facet}>{facet}</span>)}
          </div>
        </button>
      ))}
    </div>
  );
}
```

Create `web/src/components/DetailPanel.tsx`:

```tsx
import { downloadUrl } from "../api/client";
import type { InventoryItem } from "../api/types";

interface DetailPanelProps {
  item: InventoryItem | null;
}

export function DetailPanel({ item }: DetailPanelProps) {
  if (!item) {
    return <div className="empty-detail">Select a game</div>;
  }

  return (
    <div className="detail">
      <div className="large-cover" />
      <h2>{item.fileName}</h2>
      <p>{item.primaryCategory ?? "Unsorted"} · {item.organizationStatus}</p>
      <dl>
        <dt>Legacy location</dt>
        <dd>{item.legacyLocation ?? "none"}</dd>
        <dt>Version</dt>
        <dd>{item.version ?? "unknown"}</dd>
        <dt>Language</dt>
        <dd>{item.language ?? "unknown"}</dd>
      </dl>
      <div className="detail-actions">
        <button>Search DLsite</button>
        <a className="button" href={downloadUrl(item.id)}>Download</a>
      </div>
    </div>
  );
}
```

Create `web/src/components/LibraryTable.tsx`:

```tsx
import type { InventoryItem } from "../api/types";

export function LibraryTable({ items }: { items: InventoryItem[] }) {
  return (
    <table className="library-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Primary</th>
          <th>Facets</th>
          <th>Legacy</th>
          <th>Status</th>
        </tr>
      </thead>
      <tbody>
        {items.map((item) => (
          <tr key={item.id}>
            <td>{item.fileName}</td>
            <td>{item.primaryCategory ?? "Unsorted"}</td>
            <td>{item.genreFacets.join(", ")}</td>
            <td>{item.legacyLocation ?? ""}</td>
            <td>{item.organizationStatus}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
```

Create `web/src/components/ReviewQueue.tsx`:

```tsx
import type { InventoryItem } from "../api/types";

export function ReviewQueue({ items }: { items: InventoryItem[] }) {
  const pending = items.filter((item) => item.organizationStatus === "pending");
  return (
    <div className="review-list">
      {pending.map((item) => (
        <article key={item.id}>
          <strong>{item.fileName}</strong>
          <span>{item.legacyLocation ?? "no legacy"}</span>
          <button>Search DLsite</button>
        </article>
      ))}
    </div>
  );
}
```

Create `web/src/components/StagingView.tsx`:

```tsx
export function StagingView() {
  return (
    <section className="staging-view">
      <h2>Import staging</h2>
      <p>Uploaded and watched files will appear here for review before NAS organization.</p>
    </section>
  );
}
```

- [ ] **Step 2: Wire App state**

Replace `web/src/App.tsx`:

```tsx
import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { fetchLibrary } from "./api/client";
import type { InventoryItem } from "./api/types";
import { AppShell } from "./components/AppShell";
import { DetailPanel } from "./components/DetailPanel";
import { LibraryGrid } from "./components/LibraryGrid";
import { LibraryTable } from "./components/LibraryTable";
import { ReviewQueue } from "./components/ReviewQueue";

type ViewMode = "cover" | "table" | "review";

const fallbackItems: InventoryItem[] = [
  {
    id: "demo-roomgirl",
    sourceId: "demo",
    fileName: "RoomGirl V2.0.1+200.rar",
    legacyLocation: "舊 SIM+SLG",
    primaryCategory: "3D",
    genreFacets: ["Simulation", "3D"],
    organizationStatus: "pending",
    playStatus: "not_played",
    rating: null,
    version: "2.0.1",
    language: null,
    notes: null
  }
];

export function App() {
  const [viewMode, setViewMode] = useState<ViewMode>("cover");
  const [selected, setSelected] = useState<InventoryItem | null>(null);
  const query = useQuery({ queryKey: ["library"], queryFn: fetchLibrary, retry: false });
  const items = query.data?.items.length ? query.data.items : fallbackItems;
  const selectedItem = useMemo(() => selected ?? items[0] ?? null, [items, selected]);

  return (
    <AppShell detail={<DetailPanel item={selectedItem} />}>
      <header className="topbar">
        <input aria-label="Search title, filename, circle, tag, DLsite id" />
        <button>Scan</button>
        <button className="primary">Import</button>
      </header>
      <div className="view-switch">
        <button className={viewMode === "cover" ? "active" : ""} onClick={() => setViewMode("cover")}>Cover</button>
        <button className={viewMode === "table" ? "active" : ""} onClick={() => setViewMode("table")}>Table</button>
        <button className={viewMode === "review" ? "active" : ""} onClick={() => setViewMode("review")}>Review Queue</button>
      </div>
      {viewMode === "cover" && <LibraryGrid items={items} selectedId={selectedItem?.id ?? null} onSelect={setSelected} />}
      {viewMode === "table" && <LibraryTable items={items} />}
      {viewMode === "review" && <ReviewQueue items={items} />}
    </AppShell>
  );
}
```

- [ ] **Step 3: Add CSS**

Create `web/src/styles.css`:

```css
* { box-sizing: border-box; }
body {
  margin: 0;
  background: #eef1f4;
  color: #1f252d;
  font: 13px/1.45 Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  letter-spacing: 0;
}
button, input, a.button {
  font: inherit;
}
.app-shell {
  min-height: 100vh;
  display: grid;
  grid-template-columns: 220px minmax(680px, 1fr) 340px;
  background: #fff;
}
.sidebar, .detail-pane {
  background: #fbfcfd;
  border-color: #d7dce2;
  padding: 14px;
}
.sidebar { border-right: 1px solid #d7dce2; }
.detail-pane { border-left: 1px solid #d7dce2; }
.brand {
  width: 30px;
  height: 30px;
  border-radius: 7px;
  background: #1f252d;
  color: white;
  display: grid;
  place-items: center;
  font-weight: 760;
  margin-bottom: 16px;
}
nav { display: grid; gap: 4px; margin-bottom: 18px; }
nav button, .view-switch button, .topbar button, .detail-actions button, .button {
  border: 1px solid #d7dce2;
  border-radius: 6px;
  min-height: 32px;
  background: #fff;
  color: #1f252d;
  padding: 6px 10px;
  text-align: left;
  text-decoration: none;
}
nav button.active, .view-switch button.active {
  background: #e7f5f2;
  border-color: #90d3cb;
  color: #0c5c56;
  font-weight: 700;
}
.primary, .topbar button.primary {
  background: #0f766e;
  border-color: #0f766e;
  color: white;
  font-weight: 700;
}
.sidebar h3 {
  color: #697586;
  font-size: 11px;
  text-transform: uppercase;
  margin: 16px 0 7px;
}
.chips, .mini-tags { display: flex; flex-wrap: wrap; gap: 6px; }
.chip, .mini-tags span {
  border: 1px solid #d7dce2;
  border-radius: 999px;
  padding: 3px 8px;
  background: #fff;
  color: #697586;
}
.main-pane { min-width: 0; }
.topbar {
  height: 58px;
  border-bottom: 1px solid #d7dce2;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px;
}
.topbar input {
  height: 36px;
  border: 1px solid #d7dce2;
  border-radius: 7px;
  padding: 0 12px;
  flex: 1;
}
.view-switch {
  border-bottom: 1px solid #d7dce2;
  padding: 10px 16px;
  display: flex;
  gap: 8px;
}
.library-grid {
  padding: 16px;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 14px;
}
.game-card {
  border: 1px solid #d7dce2;
  border-radius: 8px;
  background: #fff;
  overflow: hidden;
  text-align: left;
  padding: 0;
}
.game-card.selected { box-shadow: 0 0 0 3px rgba(15, 118, 110, .18); border-color: #0f766e; }
.cover {
  height: 150px;
  border-bottom: 1px solid rgba(0,0,0,.08);
  padding: 8px;
}
.cover span {
  border-radius: 999px;
  background: rgba(255,255,255,.92);
  padding: 3px 7px;
  font-size: 11px;
  font-weight: 700;
}
.cover-0 { background: linear-gradient(135deg, #93c5fd, #c4b5fd 62%, #f0abfc); }
.cover-1 { background: linear-gradient(135deg, #fda4af, #fde68a 58%, #bfdbfe); }
.cover-2 { background: linear-gradient(135deg, #86efac, #67e8f9 55%, #fef08a); }
.cover-3 { background: linear-gradient(135deg, #fdba74, #f9a8d4 60%, #fef3c7); }
.cover-4 { background: linear-gradient(135deg, #a7f3d0, #fde68a 50%, #fca5a5); }
.cover-5 { background: linear-gradient(135deg, #bae6fd, #bbf7d0 58%, #fed7aa); }
.game-card strong, .game-card small, .mini-tags {
  display: block;
  margin: 8px 9px;
}
.game-card strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.game-card small { color: #697586; }
.large-cover {
  height: 190px;
  border-radius: 8px;
  background: linear-gradient(135deg, #93c5fd, #c4b5fd 62%, #f0abfc);
}
.detail h2 { font-size: 17px; line-height: 1.25; }
.detail dl {
  display: grid;
  grid-template-columns: 110px 1fr;
  gap: 8px;
}
.detail dt { color: #697586; }
.detail dd { margin: 0; }
.detail-actions { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.library-table {
  width: calc(100% - 32px);
  margin: 16px;
  border-collapse: collapse;
}
.library-table th, .library-table td {
  border-bottom: 1px solid #d7dce2;
  padding: 8px;
  text-align: left;
}
.review-list { padding: 16px; display: grid; gap: 8px; }
.review-list article {
  border: 1px solid #d7dce2;
  border-radius: 7px;
  padding: 10px;
  display: grid;
  grid-template-columns: 1fr 160px auto;
  gap: 10px;
  align-items: center;
}
```

- [ ] **Step 4: Build web UI**

Run:

```bash
cd web && npm run build
```

Expected:

```text
✓ built
```

- [ ] **Step 5: Commit**

```bash
git add web/src
git commit -m "feat: add library-first web shell"
```

## Task 13: Tauri Shell Scaffold

**Files:**
- Create: `web/src-tauri/Cargo.toml`
- Create: `web/src-tauri/build.rs`
- Create: `web/src-tauri/tauri.conf.json`
- Create: `web/src-tauri/src/main.rs`
- Modify: `web/package.json`

- [ ] **Step 1: Add Tauri package scripts**

Modify `web/package.json` scripts:

```json
{
  "dev": "vite",
  "build": "tsc -b && vite build",
  "preview": "vite preview",
  "test": "vitest run",
  "tauri": "tauri"
}
```

- [ ] **Step 2: Add Tauri config**

Create `web/src-tauri/Cargo.toml`:

```toml
[package]
name = "hgame-manager-desktop"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
```

Create `web/src-tauri/build.rs`:

```rust
fn main() {
    tauri_build::build();
}
```

Create `web/src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "HGame Manager",
  "version": "0.1.0",
  "identifier": "local.hgame.manager",
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devUrl": "http://localhost:5173",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "HGame Manager",
        "width": 1280,
        "height": 800
      }
    ]
  },
  "bundle": {
    "active": true,
    "targets": "all"
  }
}
```

Create `web/src-tauri/src/main.rs`:

```rust
fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run HGame Manager desktop app");
}
```

- [ ] **Step 3: Check Tauri build metadata**

Run:

```bash
cd web && npm install --save-dev @tauri-apps/cli && npm run tauri -- info
```

Expected: Tauri prints environment and project information.

- [ ] **Step 4: Commit**

```bash
git add web/package.json web/package-lock.json web/src-tauri
git commit -m "feat: scaffold tauri desktop shell"
```

## Task 14: Final Verification

**Files:**
- Modify only files needed to fix failures from this task.

- [ ] **Step 1: Run Rust tests**

Run:

```bash
cargo test
```

Expected:

```text
test result: ok
```

- [ ] **Step 2: Run Rust check**

Run:

```bash
cargo check
```

Expected:

```text
Finished `dev` profile
```

- [ ] **Step 3: Run web build**

Run:

```bash
cd web && npm run build
```

Expected:

```text
✓ built
```

- [ ] **Step 4: Run server with fixture config**

Run:

```bash
export DATABASE_URL=postgres://postgres:postgres@localhost/hgame_manager
export HGAME_LIBRARY_ROOT=$(pwd)/fixtures/library
cargo run -p hgame-server
```

Expected:

```text
listening on 127.0.0.1:3000
```

- [ ] **Step 5: Verify health endpoint**

Run in another shell:

```bash
curl http://127.0.0.1:3000/api/health
```

Expected:

```json
{"status":"ok"}
```

- [ ] **Step 6: Commit final fixes**

If verification required fixes, commit them:

```bash
git add Cargo.toml Cargo.lock crates web fixtures
git commit -m "fix: stabilize foundation vertical slice"
```

If no fixes were required, do not create an empty commit.

## Self-Review Notes

Spec coverage in this plan:

- Read-only NAS source: covered by `StorageRoot`, scanner, fixture tests.
- Primary category plus genre facets: covered in domain and UI.
- Legacy location: covered in scanner and UI filters.
- DLsite pluggability: covered by metadata source trait and fixture adapter.
- Download by item id: covered by Axum download route.
- Staging: covered as domain model and UI route shell.
- Export: covered as schema-versioned snapshot type.
- Library-first UI: covered by web components and CSS.
- Tauri: covered by shell scaffold.

Known planned follow-up:

- Persist scanner output into PostgreSQL through repository methods.
- Add route handlers that call scanner and repository methods instead of returning empty lists.
- Add live DLsite HTTP adapter.
- Add asset cache worker.
- Add full staging upload endpoint.
- Add RW NAS commit adapter.
- Add deployment manifests.

The follow-up items are intentionally outside this foundation slice because they are independently testable subsystems.
