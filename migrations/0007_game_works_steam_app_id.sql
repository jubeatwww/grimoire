SET search_path TO grimoire, public;

ALTER TABLE game_works
    ADD COLUMN IF NOT EXISTS steam_app_id text;

CREATE INDEX IF NOT EXISTS game_works_steam_app_id_idx ON game_works(steam_app_id);
