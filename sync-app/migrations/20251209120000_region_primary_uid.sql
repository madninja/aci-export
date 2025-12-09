-- Change regions' primary key from number → uid (matching clubs pattern)

-- 1. Drop FK from clubs.region → regions(number)
ALTER TABLE clubs
    DROP CONSTRAINT clubs_region_fkey;

-- 2. Drop FK from leadership_region.region → regions(number)
ALTER TABLE leadership_region
    DROP CONSTRAINT leadership_region_region_fkey;

-- 3. Change regions' primary key from number → uid

-- Drop old PK on number
ALTER TABLE regions
    DROP CONSTRAINT regions_pkey;

-- Make number nullable (some regions might not have a number)
ALTER TABLE regions
    ALTER COLUMN number DROP NOT NULL;

-- Make uid the new primary key
ALTER TABLE regions
    ADD CONSTRAINT regions_uid_pkey PRIMARY KEY (uid);

-- Keep number as a unique, nullable business identifier
ALTER TABLE regions
    ADD CONSTRAINT regions_number_key UNIQUE (number);

-- 4. Convert existing leadership_region data from region numbers to region uids
UPDATE leadership_region
SET region = (
    SELECT uid
    FROM regions
    WHERE regions.number = leadership_region.region
)
WHERE region IS NOT NULL;

-- 5. Create FK from leadership_region.region → regions(uid)
--    (leadership uses uid to match the entity_uid from DDB)
ALTER TABLE leadership_region
    ADD CONSTRAINT leadership_region_region_fkey
    FOREIGN KEY (region)
    REFERENCES regions(uid);

-- Note: clubs.region FK will be recreated in the next migration (20251209120001)
-- which converts clubs.region to reference regions(uid)
