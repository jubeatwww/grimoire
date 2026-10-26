SET search_path TO grimoire, public;

ALTER TABLE game_works
    ADD COLUMN IF NOT EXISTS enriched_at timestamptz;

-- Backfill: every existing row has been touched (confirm/refresh wrote columns),
-- so we treat updated_at as a reasonable proxy for "we already ran enrichment".
-- This prevents existing sparse-but-correctly-confirmed items from re-appearing
-- in the organize queue once the predicate flips to enriched_at IS NULL.
UPDATE game_works
   SET enriched_at = updated_at
 WHERE enriched_at IS NULL;
