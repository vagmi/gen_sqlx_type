create table tasks (
    id integer primary key autoincrement,
    title text not null,
    description text,
    status text not null default 'pending',
    created_at datetime default current_timestamp,
    updated_at datetime default current_timestamp
);
