-- 1. Drop the FK from members → clubs(number) so we can change clubs' PK
ALTER TABLE members
    DROP CONSTRAINT members_local_club_fkey;

-- 2. Change clubs' primary key from number → uid

-- Drop old PK on number
ALTER TABLE clubs
    DROP CONSTRAINT clubs_pkey;

-- Make number nullable (so intraclubs can have NULL)
ALTER TABLE clubs
    ALTER COLUMN number DROP NOT NULL;

-- Make uid the new primary key
ALTER TABLE clubs
    ADD CONSTRAINT clubs_uid_pkey PRIMARY KEY (uid);

-- Keep number as a unique, nullable business identifier
ALTER TABLE clubs
    ADD CONSTRAINT clubs_number_key UNIQUE (number);

-- 3. Recreate the FK from members.local_club → clubs.number

ALTER TABLE members
    ADD CONSTRAINT members_local_club_fkey
    FOREIGN KEY (local_club)
    REFERENCES clubs(number);-- Add migration script here
