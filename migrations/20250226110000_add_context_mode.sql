DO $$ BEGIN
    CREATE TYPE context_mode AS ENUM ('emb', 'rdf', 'lpg');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

ALTER TABLE contexts
ADD COLUMN IF NOT EXISTS mode context_mode NOT NULL DEFAULT 'rdf';

UPDATE contexts SET mode = 'rdf' WHERE mode IS NULL;
