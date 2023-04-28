create table message
(
    id   bigserial primary key,
    time timestamp with time zone not null,
    text varchar(65535)           not null
);

create index message_time_idx on message (time);
