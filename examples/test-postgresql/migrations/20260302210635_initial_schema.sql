begin;
create table tasks (
    id bigserial primary key,
    title text not null,
    description text,
    status text not null default 'pending',
    created_at timestamptz default current_timestamp,
    updated_at timestamptz default current_timestamp
);

commit;
