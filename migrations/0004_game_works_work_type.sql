SET search_path TO grimoire, public;

ALTER TABLE game_works
    ADD COLUMN IF NOT EXISTS work_type text,
    ADD COLUMN IF NOT EXISTS work_type_label text;
