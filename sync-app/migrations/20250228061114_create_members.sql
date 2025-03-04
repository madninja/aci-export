create table users (
    email text primary key,
    uid bigint not null,
    first_name text,
    last_name text,
    birthday date,
    state text,
    phone_mobile text,
    phone_home text,
    last_login timestamptz
);

create table regions (
    number bigint primary key,
    uid bigint not null,
    name text
);

create table clubs (
    number bigint primary key,
    uid bigint not null,
    name text,
    region bigint references regions(number)
);

create type member_class as enum ('regular','lifetime');
create type member_type as enum ('regular', 'affiliate');

create table members (
    primary_user text primary key references users(email),
    partner_user text references users(email),
    member_class member_class,
    member_type member_type,
    expiration_date date,
    join_date date,
    local_club bigint references clubs(number)
);

create table addresses (
    user_id text references users(email),
    street_address text,
    street_address_2 text,
    zip_code text,
    city text,
    state text,
    country text
);

create table brns (
    number text primary key,
    uid bigint not null,
    owner text references members(primary_user) 
);

alter table users enable row level security;
alter table regions enable row level security;
alter table clubs enable row level security;
alter table members enable row level security;
alter table brns enable row level security;

create policy "users is visible only to authenticated users"
on users for select to authenticated using ( true );

create policy "regions is visible only to authenticated users"
on regions for select to authenticated using ( true );

create policy "clubs is visible only to authenticated users"
on clubs for select to authenticated using ( true );

create policy "members is visible only to authenticated users"
on members for select to authenticated using ( true );

create policy "brns is visible only to authenticated users"
on brns for select to authenticated using ( true );
-- Add migration script here
