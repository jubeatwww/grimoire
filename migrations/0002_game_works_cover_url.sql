SET search_path TO grimoire, public;

ALTER TABLE game_works
    ADD COLUMN IF NOT EXISTS cover_image_url text;
