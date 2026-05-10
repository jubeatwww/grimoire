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
