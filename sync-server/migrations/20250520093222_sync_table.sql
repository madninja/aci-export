create table mailchimp  (
    id bigint primary key generated always as identity,
    name text,
    api_key text not null unique,
    list text not null unique,

    club bigint,
    region int,
    created_at timestamptz default now()
);

