create table mailchimp  (
    id bigint primary key generated always as identity,
    name text,
    api_key text not null unique,
    list text not null unique,

    club bigint,
    region int,
    created_at timestamptz default now()
);

alter table mailchimp enable row level security;

create policy "mailchimp is visible only to authenticated users"
on mailchimp for select to authenticated using ( true );
