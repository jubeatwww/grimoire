SET search_path TO grimoire, public;

ALTER TABLE game_works
    ADD COLUMN IF NOT EXISTS preview_image_urls jsonb NOT NULL DEFAULT '[]',
    ADD COLUMN IF NOT EXISTS file_type text,
    ADD COLUMN IF NOT EXISTS file_size_bytes bigint,
    ADD COLUMN IF NOT EXISTS dl_count integer,
    ADD COLUMN IF NOT EXISTS rate_average real,
    ADD COLUMN IF NOT EXISTS rate_count integer,
    ADD COLUMN IF NOT EXISTS price_jpy integer;
