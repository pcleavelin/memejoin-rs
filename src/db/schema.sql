BEGIN;

create table if not exists User
(
    username TEXT not null
        constraint User_pk
            primary key,
    api_key TEXT not null,
    api_key_expires_at DATETIME not null,
    discord_token TEXT not null,
    discord_token_expires_at DATETIME not null
);

create table if not exists Intro
(
    id     integer not null
        constraint Intro_pk
            primary key autoincrement,
    name   TEXT    not null,
    volume integer not null,
    guild_id integer not null
        constraint Intro_Guild_guild_id_fk
            references Guild ("id"),
    filename   TEXT    not null
);

create table if not exists Guild
(
    id          integer    not null
        primary key,
    name        TEXT    not null,
    sound_delay integer not null
);

create table if not exists Channel
(
    name     TEXT
        primary key,
    guild_id integer
        constraint Channel_Guild_id_fk
            references Guild (id)
);

create table if not exists UserGuild
(
    username TEXT not null
        constraint UserGuild_User_username_fk
            references User,
    guild_id integer not null
        constraint UserGuild_Guild_id_fk
            references Guild (id),
    primary key ("username", "guild_id")
);

create table if not exists UserIntro
(
    username     text    not null
        constraint UserIntro_User_username_fk
            references User,
    intro_id     integer not null
        constraint UserIntro_Intro_id_fk
            references Intro,
    guild_id     integer    not null
        constraint UserIntro_Guild_guild_id_fk
            references Guild ("id"),
    channel_name text    not null
        constraint UserIntro_Channel_channel_name_fk
            references Channel ("name"),
    primary key ("username", "intro_id", "guild_id", "channel_name")
);

create table if not exists UserPermission
(
    username    TEXT    not null
        constraint UserPermission_User_username_fk
            references User,
    guild_id integer not null
        constraint User_Guild_guild_id_fk
            references Guild ("id"),
    permissions integer not null,
    primary key ("username", "guild_id")
);

create table if not exists UserAppPermission
(
    username    TEXT    not null
        constraint UserPermission_User_username_fk
            references User,
    permissions integer not null,
    primary key ("username")
);

COMMIT;
