create table users (
    id text primary key,
    email text unique not null,
    uid bigint not null,
    first_name text,
    last_name text,
    birthday date,
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
    primary_user text primary key references users(id),
    partner_user text references users(id),
    member_class member_class,
    member_type member_type,
    expiration_date date,
    join_date date,
    local_club bigint references clubs(number)
);

create table addresses (
    user_id text primary key references users(id),
    street_address text,
    street_address_2 text,
    zip_code text,
    city text,
    state text,
    country text
);

create table brns (
    number text primary key,
    user_id text references members(primary_user)
);

alter table users enable row level security;
alter table regions enable row level security;
alter table clubs enable row level security;
alter table members enable row level security;
alter table addresses enable row level security;
alter table brns enable row level security;

