-- Remove PII data and restrict region write access for security
-- This migration addresses data exposure concerns with Supabase anon key access

-- ============================================
-- USERS: Drop PII columns (keeping email for now)
-- ============================================
ALTER TABLE users DROP COLUMN IF EXISTS birthday;
ALTER TABLE users DROP COLUMN IF EXISTS phone_mobile;
ALTER TABLE users DROP COLUMN IF EXISTS phone_home;
ALTER TABLE users DROP COLUMN IF EXISTS last_login;

-- ============================================
-- ADDRESSES: Drop detailed location columns
-- ============================================
ALTER TABLE addresses DROP COLUMN IF EXISTS street_address;
ALTER TABLE addresses DROP COLUMN IF EXISTS street_address_2;
ALTER TABLE addresses DROP COLUMN IF EXISTS zip_code;
ALTER TABLE addresses DROP COLUMN IF EXISTS city;

-- ============================================
-- REGIONS: Lock down to read-only for public
-- ============================================
-- Drop the overly permissive policies
DROP POLICY IF EXISTS "Enable delete for authenticated users only" ON regions;
DROP POLICY IF EXISTS "Enable insert for authenticated users only" ON regions;
DROP POLICY IF EXISTS "Enable update for authenticated users only" ON regions;

-- Create service_role only policies for writes
CREATE POLICY "Service role can insert regions" ON regions
  FOR INSERT TO service_role
  WITH CHECK (true);

CREATE POLICY "Service role can update regions" ON regions
  FOR UPDATE TO service_role
  USING (true);

CREATE POLICY "Service role can delete regions" ON regions
  FOR DELETE TO service_role
  USING (true);
