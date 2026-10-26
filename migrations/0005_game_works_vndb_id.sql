SET search_path TO grimoire, public;

ALTER TABLE game_works
    ADD COLUMN IF NOT EXISTS vndb_id text;

CREATE INDEX IF NOT EXISTS game_works_vndb_id_idx ON game_works(vndb_id);
