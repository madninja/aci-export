# SSP Membership DB — Database Schema Primer

This document contains the complete database schema, business logic, and query
patterns for SSP membership data. It is designed to be used as a reference by
LLMs generating SQL queries (see `ddb_prompt.md` for behavioral instructions)
or by humans learning the database structure.

---

## Core Tables

### **users_field_data**
- Canonical user table
- Fields:
  - `uid` - User ID
  - `mail` - Email address
  - `pass` - Password hash (PHPass format, see below)
  - `login` - Last login (UNIX timestamp)
- Also used for partner last login via `partner_user_id`.

### Password Hashing (PHPass)

Drupal stores passwords in the `pass` field using PHPass SHA-512:

```
$S$E[8-char salt][43-char hash]
 │ │
 │ └── Iteration count: position in ./0-9A-Za-z alphabet → 2^position
 └──── Algorithm identifier: SHA-512
```

Common iteration characters:
- `D` = 2^15 = 32,768 iterations
- `E` = 2^16 = 65,536 iterations (most common in Drupal 8+)

Verification algorithm:
1. Extract salt (chars 4-11) and expected hash (chars 12-54)
2. Compute: `hash = SHA512(salt + password)`
3. Iterate: `hash = SHA512(hash + password)` for N iterations
4. Encode result with Drupal's base64 alphabet: `./0-9A-Za-z`
5. Compare first 43 chars to expected hash

### **z_member_search_main**
One row per user (materialized profile + partner info):
- `first_name`, `last_name`, `email`
- `birthdate`
- `personal_status_id`
- Partner fields:
  - `partner_user_id`
  - `partner_first_name`, `partner_last_name`
  - `partner_email`
  - `partner_birthdate`

---

## Membership Paragraphs

Membership exists in **paragraph entities**:

### **paragraphs_item_field_data**
- `id` (paragraph ID)
- `parent_id` (user UID)
- `type = 'membership'`
- `status = '1'`

### Attached fields:
- **paragraph__field_club**
  - `field_club_target_id` → club nid
- **paragraph__field_join_date**
- **paragraph__field_leave_date**
- **paragraph__field_membership_class**
  - taxonomy term via `taxonomy_term_field_data`

### Active membership rule:
A membership paragraph is active if:
- `join_date <= CURRENT_DATE`
- and `(leave_date IS NULL OR leave_date >= CURRENT_DATE)`

---

## Clubs and Regions

### **Clubs** (nodes)
`node_field_data WHERE type = 'ssp_club'`
- `nid` (uid)
- `title` (name)
- `status` (0 = unpublished/inactive, 1 = published/active)

Additional club metadata:
- `node__field_club_number`
- `node__field_region` → region nid

**Important**: Query clubs directly from `node_field_data` (not from `paragraph__field_club`). The paragraph table only contains clubs that have members, missing inactive/deleted clubs that may still have historical leadership records.

### **Regions** (also nodes)
`node_field_data WHERE type = 'ssp_region'`
- `nid` (uid)
- `title` (name)
- `status` (0 = unpublished/inactive, 1 = published/active)

Additional region metadata:
- `node__field_region_number`

**Note**: Like clubs, regions have a `status` field that indicates whether they are active. Include this when syncing to preserve inactive region information.

---

## Membership Associations

These link users → membership paragraphs:

- **user__field_home_club**
  - home (primary) membership
- **user__field_memberships**
  - affiliate membership
- **user__field_intraclub_memberships**
  - intraclub membership

These tables reference the **paragraph id**, not the club id.

---

## Member Type Classification

### Output field: `member_type`
Rules:
- `'regular'` = has **home** or **intraclub** membership
- `'affiliate'` = has affiliate membership

From flags aggregated across active paragraphs:
```sql
CASE
  WHEN member_flag = 1 THEN 'regular'
  WHEN affiliate_flag = 1 THEN 'affiliate'
END
```

---

## Partner Model

### Relationship table:
**user__field_primary_member**
- `entity_id` = partner uid
- `field_primary_member_target_id` = primary uid
- `deleted = '0'`

### Critical: Partners Are Not in z_member_search_main
**Partner users do NOT appear as separate rows in `z_member_search_main`.**

- Only primary members appear in `z_member_search_main`
- Partner demographic data is stored in the **primary member's row** via partner_* fields
- Partner membership status is determined by their primary member's status
- Partner user accounts exist in `users_field_data` with their own UID
- Primary users must **exclude** users who appear as partner-only

### Querying partner users:
To get complete user data including partners:
1. Always JOIN to `users_field_data` for basic user info (uid, mail, login)
2. LEFT JOIN to `z_member_search_main` for member demographics
3. Use `COALESCE(md.email, usr.mail)` to fall back to `users_field_data` when partner
4. Check `user__field_primary_member` to identify if user is a partner
5. If partner, look up primary member's membership status

### Data sources:
- `z_member_search_main` for **primary member** demographics only
- `users_field_data` for all users including partners
- Primary member's row contains partner fields:
  - `partner_uid`
  - `partner_first_name`
  - `partner_last_name`
  - `partner_email`
  - `partner_birthday`
  - `partner_last_login`

---

## Status Filtering

Only include members with:
```
personal_status_id IN ('947', '951', '1099')
```

---

## Selecting the User’s Current (Most Recent) Active Membership

### Step 1: `acp` CTE
Filters paragraphs by:
- active membership
- membership type = 'membership'

### Step 2: `flags` CTE
Aggregates per user:
- `member_flag`
- `affiliate_flag`
- `latest_join_date`
- `latest_expiration_date`

### Step 3: `active_pick` CTE
Chooses **one** active membership paragraph per user:
- The one with the **latest join date**

This defines the user’s “current club context.”

---

## Scope CTE — Club, Region, or All-Member Queries

Unified approach using:

### `scope` CTE
Contains club IDs relevant to the query:
- If club_nid is provided → 1 club
- If region_nid is provided → all clubs in region
- If both NULL → `scope` is empty

### Membership filtering:
```sql
AND (
  NOT EXISTS (SELECT 1 FROM scope)
  OR club_nid IN (SELECT club_nid FROM scope)
)
```

Therefore:
- **Club mode**: filter to that club
- **Region mode**: filter to all clubs in region
- **All-members mode**: no filtering

---

## Output Field Conventions

### Primary user fields:
- `uid`
- `last_login`
- `first_name`
- `last_name`
- `email`
- `birthday`

### Member info fields:
- `member_type` ('regular' or 'affiliate')
- `member_class` (taxonomy term or `'Regular'`)
- `member_status` (personal_status_id)
- `join_date`
- `expiration_date`

### Club fields (from `active_pick`):
- `club_uid`
- `club_name`
- `club_number`
- `club_region`
- `club_region_uid`
- `brns`

### Partner fields:
- `partner_uid`
- `partner_last_login`
- `partner_first_name`
- `partner_last_name`
- `partner_email`
- `partner_birthday`

---

## Leadership Model

Leadership positions for clubs, intraclubs, and regions are stored using paragraph entities.

### Tables

**node__field_leadership_ssp**
- Links clubs/regions to leadership paragraph entities
- `entity_id` = club or region nid
- `field_leadership_ssp_target_id` = paragraph id
- `delta` = position order (0, 1, 2, ...) allows multiple leaders per entity

**paragraphs_item_field_data**
- `type = 'ssp_leadership_region'` (for both clubs and regions)
- Each paragraph represents one person in one role

### Leadership paragraph fields:

**paragraph__field_role**
- `field_role_target_id` → taxonomy term id (tid)
- References `taxonomy_term_field_data` for role name
- Common roles:
  - **Current positions**: President (940), 1st Vice President (929), 2nd Vice President (930), Treasurer (944), Membership Chair (932), Recording Secretary (941), Corresponding Secretary (931), Newsletter Editor (933), Webmaster (945)
  - **Advisory positions**: Past President (936), Past Vice President (939), Past Treasurer (938), Past Membership Chairman (935), Past Recording Secretary (937), Past Corresponding Sec. (934)
  - Note: "Past" roles are **current board positions** (advisory), not historical tracking

**paragraph__field_user** or **paragraph__field_member**
- `field_user_target_id` or `field_member_target_id` → user uid
- References the person holding the position
- Can be primary members, partners, or non-members

**paragraph__field_start_date**
- When the person started this role

**paragraph__field_end_date**
- When the person ended this role
- NULL = currently serving

### Active leadership rule:
A leadership position is currently active if:
- `start_date <= CURRENT_DATE`
- and `(end_date IS NULL OR end_date >= CURRENT_DATE)`

### Important characteristics:

1. **No schema-enforced term lengths**: Term durations are managed organizationally (bylaws, elections), not by database constraints
2. **Multiple roles allowed**: Users can hold multiple leadership positions simultaneously (tracked via multiple paragraph records)
3. **Historical tracking**: Past assignments tracked via same structure with filled end_date values
4. **Partners and non-members can lead**: Leadership positions can be held by:
   - Primary members (in z_member_search_main)
   - Partner members (NOT in z_member_search_main, must join via users_field_data)
   - Non-members (user accounts with no membership paragraphs)

### Node types with leadership:

Leadership data exists for multiple node types:
- **ssp_club** (126 entities) - clubs and intraclubs
- **ssp_region** (13 entities) - regions
- **ssp_standing_committees** (20 entities) - committees
- **ssp_international_leadership** (1 entity) - international leadership

When querying leadership for clubs/regions specifically, **always filter by node type** to exclude committees and international leadership.

### Querying leadership:

```sql
-- Current leadership for clubs and regions only
SELECT
    entity.nid AS entity_uid,
    entity.title AS entity_name,
    entity.type AS entity_type,
    role_term.name AS role,
    usr.uid,
    COALESCE(md.email, usr.mail) AS email,
    DATE(start.field_start_date_value) AS start_date
FROM node_field_data entity
JOIN node__field_leadership_ssp l
    ON l.entity_id = entity.nid AND l.deleted = '0'
JOIN paragraphs_item_field_data p
    ON p.id = l.field_leadership_ssp_target_id
LEFT JOIN paragraph__field_role r ON r.entity_id = p.id AND r.deleted = '0'
LEFT JOIN taxonomy_term_field_data role_term ON role_term.tid = r.field_role_target_id
LEFT JOIN paragraph__field_start_date start ON start.entity_id = p.id AND start.deleted = '0'
LEFT JOIN paragraph__field_end_date end ON end.entity_id = p.id AND end.deleted = '0'
LEFT JOIN paragraph__field_user u ON u.entity_id = p.id AND u.deleted = '0'
LEFT JOIN paragraph__field_member m ON m.entity_id = p.id AND m.deleted = '0'
JOIN users_field_data usr ON usr.uid = COALESCE(u.field_user_target_id, m.field_member_target_id)
LEFT JOIN z_member_search_main md ON md.user_id = usr.uid
WHERE entity.type IN ('ssp_club', 'ssp_region')  -- IMPORTANT: filter to clubs/regions only
  AND DATE(start.field_start_date_value) <= CURRENT_DATE
  AND (end.field_end_date_value IS NULL OR DATE(end.field_end_date_value) >= CURRENT_DATE);

-- For a specific entity, replace the type filter with:
-- WHERE entity.nid = ?

-- International leadership only
SELECT
    entity.nid AS entity_uid,
    entity.title AS entity_name,
    role_term.name AS role,
    usr.uid,
    COALESCE(md.email, usr.mail) AS email,
    DATE(start.field_start_date_value) AS start_date
FROM node_field_data entity
JOIN node__field_leadership_ssp l
    ON l.entity_id = entity.nid AND l.deleted = '0'
JOIN paragraphs_item_field_data p
    ON p.id = l.field_leadership_ssp_target_id
LEFT JOIN paragraph__field_role r ON r.entity_id = p.id AND r.deleted = '0'
LEFT JOIN taxonomy_term_field_data role_term ON role_term.tid = r.field_role_target_id
LEFT JOIN paragraph__field_start_date start ON start.entity_id = p.id AND start.deleted = '0'
LEFT JOIN paragraph__field_end_date end ON end.entity_id = p.id AND end.deleted = '0'
LEFT JOIN paragraph__field_user u ON u.entity_id = p.id AND u.deleted = '0'
LEFT JOIN paragraph__field_member m ON m.entity_id = p.id AND m.deleted = '0'
JOIN users_field_data usr ON usr.uid = COALESCE(u.field_user_target_id, m.field_member_target_id)
LEFT JOIN z_member_search_main md ON md.user_id = usr.uid
WHERE entity.type = 'ssp_international_leadership'
  AND DATE(start.field_start_date_value) <= CURRENT_DATE
  AND (end.field_end_date_value IS NULL OR DATE(end.field_end_date_value) >= CURRENT_DATE);
```

---

## Query Guidance for LLMs

### For membership queries:
1. Always start from **active membership paragraphs**.
2. Use paragraph-level join/leave dates for membership validity.
3. Determine member type using:
   - `user__field_home_club`
   - `user__field_intraclub_memberships`
   - `user__field_memberships`
4. Use the latest join date to select **one** active membership per user.
5. Use `z_member_search_main` for profile + partner info.
6. Use `users_field_data` for primary + partner login timestamps.
7. Apply the `scope` CTE to support:
   - club queries
   - region queries
   - all-member queries
8. Exclude partner-only users when querying members.
9. Alias birthdays as `birthday` and `partner_birthday`.

### For queries involving partners:
1. **Always JOIN to `users_field_data`** for basic user info (uid, mail, login)
2. **LEFT JOIN to `z_member_search_main`** - it won't have partner rows
3. **Use COALESCE** to fall back: `COALESCE(md.email, usr.mail)`
4. Check `user__field_primary_member` to identify partners
5. If partner, query primary member for membership status

### For leadership queries:
1. Start from `node__field_leadership_ssp` linked to entity (club/region/international)
2. **Always filter by node type**: `WHERE entity.type IN ('ssp_club', 'ssp_region')`
   - Without this filter, queries will include committees and international leadership
   - Node types with leadership: ssp_club, ssp_region, ssp_standing_committees, ssp_international_leadership
3. Join through `paragraphs_item_field_data` to leadership paragraphs
4. Use start/end dates with same active logic as membership
5. **Must JOIN to `users_field_data`** (leaders may not be in z_member_search_main)
6. **LEFT JOIN to `z_member_search_main`** and use COALESCE for names/emails
7. Join to `taxonomy_term_field_data` for role names
8. Multiple leadership roles per person are normal (no deduplication needed)
9. When querying international leadership specifically, use `entity.type = 'ssp_international_leadership'`

---

## DDB Data Quality Issues

When syncing leadership data, handle these defensive cases:

### Null start_date
Some leadership records have null `start_date`. Filter these out:
```sql
WHERE start.field_start_date_value IS NOT NULL
```

### Null role
Some leadership records have null `role_uid`. Handle or filter as needed.

### Duplicate composite keys
DDB can have duplicate leadership entries with the same composite key. Deduplicate before upserting:
- Club leadership: `(club_uid, user_id, role_uid, start_date)`
- Region leadership: `(region_uid, user_id, role_uid, start_date)`
- International leadership: `(user_id, role_uid, start_date)`

### Inactive/unpublished clubs and regions
Leadership records can reference clubs/regions with `status = 0` (unpublished/inactive). These entities exist in `node_field_data` but may not appear in queries that filter by status. Include all clubs/regions regardless of status when syncing to ensure foreign key integrity.

When syncing clubs and regions, always include the `status` field (mapped to `active` boolean) so downstream systems can distinguish active from inactive entities:
```sql
SELECT
    nd.nid as uid,
    nd.title as name,
    nd.status as active,  -- 1 = active, 0 = inactive
    ...
FROM node_field_data nd
WHERE nd.type = 'ssp_club'  -- or 'ssp_region'
```

---

## End of Primer
