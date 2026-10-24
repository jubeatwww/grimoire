SET search_path TO grimoire, public;

-- One row per executed move batch; undo replays its ops in reverse.
CREATE TABLE IF NOT EXISTS move_batches (
    id           uuid PRIMARY KEY,
    status       text NOT NULL CHECK (status IN ('running', 'done', 'undoing', 'undone')),
    moved        integer NOT NULL DEFAULT 0,
    skipped      integer NOT NULL DEFAULT 0,
    failed       integer NOT NULL DEFAULT 0,
    removed_dirs jsonb NOT NULL DEFAULT '[]',
    notes        jsonb NOT NULL DEFAULT '[]',
    created_at   timestamptz NOT NULL DEFAULT now(),
    finished_at  timestamptz,
    undone_at    timestamptz
);

CREATE TABLE IF NOT EXISTS move_ops (
    id                uuid PRIMARY KEY,
    batch_id          uuid NOT NULL REFERENCES move_batches(id) ON DELETE CASCADE,
    seq               integer NOT NULL,
    inventory_item_id uuid REFERENCES inventory_items(id) ON DELETE SET NULL,
    from_path         text NOT NULL,
    to_path           text NOT NULL,
    -- pending / undo_pending are written before a file is touched, so an
    -- interrupted batch can be reconciled against the disk afterwards.
    status            text NOT NULL CHECK (status IN (
                          'pending', 'moved', 'skipped', 'failed',
                          'undo_pending', 'undone', 'undo_skipped', 'undo_failed')),
    message           text,
    updated_at        timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS move_ops_batch_idx ON move_ops (batch_id, seq);
