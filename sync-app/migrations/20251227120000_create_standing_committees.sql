-- Create standing_committees table
create table standing_committees (
    uid bigint primary key,
    name text not null,
    active boolean not null default true
);

create index idx_standing_committees_active on standing_committees(active);
alter table standing_committees enable row level security;

-- Create standing committee leadership table
create table leadership_standing_committee (
    id bigserial primary key,
    standing_committee bigint not null references standing_committees(uid),
    user_id text not null references users(id),
    role bigint not null references leadership_role(uid),
    start_date date not null,
    end_date date,
    unique(standing_committee, user_id, role, start_date)
);

create index idx_leadership_standing_committee_committee on leadership_standing_committee(standing_committee);
create index idx_leadership_standing_committee_user on leadership_standing_committee(user_id);
create index idx_leadership_standing_committee_role on leadership_standing_committee(role);
alter table leadership_standing_committee enable row level security;
