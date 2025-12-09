-- Create leadership_role table (no dependencies)
create table leadership_role (
    uid bigint primary key,
    title text not null
);

-- Create leadership_club table (depends on clubs, users, leadership_role)
create table leadership_club (
    id bigserial primary key,
    club bigint not null references clubs(uid),
    user_id text not null references users(id),
    role bigint not null references leadership_role(uid),
    start_date date not null,
    end_date date,
    constraint unique_club_leadership unique (club, user_id, role, start_date)
);

-- Create leadership_region table (depends on regions, users, leadership_role)
create table leadership_region (
    id bigserial primary key,
    region bigint not null references regions(number),
    user_id text not null references users(id),
    role bigint not null references leadership_role(uid),
    start_date date not null,
    end_date date,
    constraint unique_region_leadership unique (region, user_id, role, start_date)
);

-- Create leadership_international table (no entity foreign key, only users and role)
create table leadership_international (
    id bigserial primary key,
    user_id text not null references users(id),
    role bigint not null references leadership_role(uid),
    start_date date not null,
    end_date date,
    constraint unique_international_leadership unique (user_id, role, start_date)
);

-- Add indexes for query performance
create index idx_leadership_club_club on leadership_club(club);
create index idx_leadership_club_user_id on leadership_club(user_id);
create index idx_leadership_club_role on leadership_club(role);

create index idx_leadership_region_region on leadership_region(region);
create index idx_leadership_region_user_id on leadership_region(user_id);
create index idx_leadership_region_role on leadership_region(role);

create index idx_leadership_international_user_id on leadership_international(user_id);
create index idx_leadership_international_role on leadership_international(role);

-- Enable row level security
alter table leadership_role enable row level security;
alter table leadership_club enable row level security;
alter table leadership_region enable row level security;
alter table leadership_international enable row level security;
