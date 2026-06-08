-- Migration 002: Add missing columns to workflows table
-- These columns are needed by the application but were missing from the initial schema

-- Add current_plan_id and current_task_id to workflows table
ALTER TABLE workflows ADD COLUMN IF NOT EXISTS current_plan_id TEXT;
ALTER TABLE workflows ADD COLUMN IF NOT EXISTS current_task_id TEXT;

-- Add index for current_plan_id for faster lookups
CREATE INDEX IF NOT EXISTS idx_workflows_current_plan_id ON workflows(current_plan_id);
CREATE INDEX IF NOT EXISTS idx_workflows_current_task_id ON workflows(current_task_id);
