DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_enum e
        JOIN pg_type t ON t.oid = e.enumtypid
        WHERE t.typname = 'member_class'
          AND e.enumlabel = 'complimentary'
    ) THEN
        ALTER TYPE member_class ADD VALUE 'complimentary';
    END IF;
END
$$;
