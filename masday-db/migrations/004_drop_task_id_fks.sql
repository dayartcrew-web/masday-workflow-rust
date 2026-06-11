-- Migration 004: Drop FK constraints on task_id referencing tasks table
-- task_id fields accept custom identifiers (not just tasks.id), so FK is inappropriate.

DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT conname, conrelid::regclass AS tbl
        FROM pg_constraint
        WHERE contype = 'f'
          AND confrelid = 'tasks'::regclass
          AND conname LIKE '%task_id_fkey'
    LOOP
        EXECUTE format('ALTER TABLE %s DROP CONSTRAINT %I', r.tbl, r.conname);
    END LOOP;
END $$;
