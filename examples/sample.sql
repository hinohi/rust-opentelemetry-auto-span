DROP DATABASE IF EXISTS sample;
CREATE DATABASE sample;
USE sample;

DROP TABLE IF EXISTS users;
CREATE TABLE users
(
    id       bigint primary key,
    name     varchar(100) not null,
    language varchar(100)
);

INSERT INTO users
VALUES (1, 'ferris', 'Rust'),
       (2, 'Gopher', 'Go'),
       (3, 'snake', 'Python'),
       (4, 'D', 'D');
