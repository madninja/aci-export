-- Change clubs.region to reference regions(uid) instead of regions(number)

-- 1. Drop the existing FK from clubs.region → regions(number)
ALTER TABLE clubs
    DROP CONSTRAINT clubs_region_fkey;

-- 2. For existing data: convert region numbers to region uids
--    This handles any existing clubs that have region numbers stored
UPDATE clubs
SET region = (
    SELECT uid
    FROM regions
    WHERE regions.number = clubs.region
)
WHERE region IS NOT NULL;

-- 3. Create new FK from clubs.region → regions(uid)
ALTER TABLE clubs
    ADD CONSTRAINT clubs_region_fkey
    FOREIGN KEY (region)
    REFERENCES regions(uid);
