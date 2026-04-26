ALTER TABLE contexts ADD COLUMN owner_id UUID;
CREATE INDEX contexts_owner_id_idx ON contexts (owner_id);
